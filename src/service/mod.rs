use crate::error::AppError;
use anyhow::anyhow;
use api_types::AnalyzeResultOperation;
use chrono::TimeZone;
use chrono_tz::Europe::Copenhagen;
use itertools::Itertools as _;
use serde::Serialize;
use sqlx::PgPool;

mod api_types;
pub mod ocr;

// Postgres maximum number of parameters in a statement
const BIND_LIMIT: usize = 65535;

pub async fn process_analysis_results(
    file_hash: &str,
    analysis_result: AnalyzeResultOperation,
    pool: &PgPool,
) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT INTO raw_results(sha256_digest, result_json) VALUES ($1, $2)",
        file_hash,
        serde_json::to_string(&analysis_result)?
    )
    .execute(pool)
    .await?;
    tracing::info!("Successfully cached raw response text in DB. Processing further...");
    save_analysis_data(pool, analysis_result, file_hash).await?;
    tracing::info!("Successfully saved receipt data in database");
    Ok::<(), AppError>(())
}

pub async fn save_analysis_data(
    pool: &PgPool,
    analysis_result: AnalyzeResultOperation,
    file_hash: &str,
) -> Result<(), AppError> {
    let receipt_fields = analysis_result
        .analyzeResult
        .ok_or(anyhow!("Missing analyzeResult field"))?
        .documents
        .ok_or(anyhow!("Missing documents field"))?
        .first()
        .ok_or(anyhow!("Documents field is present but empty"))?
        .fields
        .clone();
    let merchant_name = &receipt_fields.merchant_name.value_string;
    let (product_names, (counts, unit_prices)): (Vec<_>, (Vec<_>, Vec<_>)) =
        if let None = receipt_fields.items {
            (
                vec![merchant_name.clone()],
                (vec![1.0], vec![receipt_fields.total.value_currency.amount]),
            )
        } else {
            receipt_fields
                .items
                .unwrap()
                .value_array
                .iter()
                .filter_map(|item| {
                    let Some(unit_price) = item
                        .value_object
                        .unit_price
                        .as_ref()
                        .or(item.value_object.total_price.as_ref())
                        .map(|obj| obj.value_currency.amount)
                    else {
                        // We throw away items where no price was detected
                        return None;
                    };
                    let name = item.value_object.description.value_string.clone();
                    let count = if let Some(q) = &item.value_object.quantity {
                        q.value_number
                    } else {
                        1.0
                    };
                    Some((name, (count, unit_price)))
                })
                .into_iter()
                .take(BIND_LIMIT)
                .unzip()
        };

    // Netto receipt date strings detected by analysis API are usually well formatted (YYYY-m-d), but when generating a date value from that the model tends to flip month and day;
    // TODO: For now Netto dates will be a special case, until similar issue is encountered elsewhere
    let date_str = if receipt_fields.transaction_date.content.contains("-")
        && merchant_name.to_lowercase().contains("netto")
    {
        receipt_fields.transaction_date.content
    } else {
        receipt_fields.transaction_date.value_date
    };

    let datetime_str = date_str + " " + &receipt_fields.transaction_time.value_time;
    let timestamp = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| anyhow!(format!("Invalid date string: {datetime_str}")))?;
    let chrono::LocalResult::Single(timestamp_tz) = Copenhagen.from_local_datetime(&timestamp)
    else {
        return Err(anyhow!("Error converting naive timestamp to Copenhagen time").into());
    };

    let tx = pool.begin().await?;

    // TODO: Currently the entire transaction crashes if there already exists a receipt with identical timestamp; in real life it would be possible for that to happen (especially if there is a lot of users)
    let receipt_id =
        insert_receipt_if_not_exists(pool, merchant_name, timestamp_tz, file_hash).await?;

    insert_products_if_not_exist(pool, &product_names)
        .await
        .map_err(AppError::from)?;

    upsert_prices_for_products_and_receipt(pool, counts, unit_prices, product_names, receipt_id)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn upsert_prices_for_products_and_receipt(
    pool: &PgPool,
    counts: Vec<f64>,
    unit_prices: Vec<f64>,
    product_names: Vec<String>,
    receipt_id: i32,
) -> Result<(), sqlx::Error> {
    // TODO: De-duplication means we are losing data points such as multiple discounts with the same name on one receipt; allow multiple entries of a given product on the same receipt
    let mut data = product_names
        .into_iter()
        .zip(counts.into_iter().zip(unit_prices.into_iter()))
        .unique_by(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    data.sort_by(|(name1, _), (name2, _)| name1.cmp(name2));
    let (product_names, (counts, unit_prices)): (Vec<String>, (Vec<f64>, Vec<f64>)) =
        data.into_iter().unzip();
    sqlx::query!(
        r#"INSERT INTO prices(count, unit_price, receipt_id, product_id) SELECT tmp.count, tmp.unit_price, tmp.receipt_id, products.id FROM (SELECT UNNEST($1::float[]) AS count, UNNEST($2::float[]) AS unit_price, $3::integer AS receipt_id, UNNEST($4::text[]) AS name) tmp INNER JOIN products ON tmp.name = products.name ON CONFLICT ON CONSTRAINT prices_pkey DO UPDATE SET count=excluded.count, unit_price=excluded.unit_price"#,
        &counts,
        &unit_prices,
        receipt_id,
        &product_names
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_receipt_if_not_exists(
    pool: &PgPool,
    merchant_name: &str,
    paid_at: chrono::DateTime<chrono_tz::Tz>,
    file_hash: &str,
) -> Result<i32, sqlx::Error> {
    let res = sqlx::query!(
        r#"INSERT INTO receipts(merchant_name, paid_at, file_sha256) VALUES ($1, $2, $3) RETURNING *"#,
        merchant_name,
        paid_at,
        file_hash
    )
    .fetch_one(pool)
    .await?
    .id;
    Ok(res)
}

pub async fn insert_products_if_not_exist(
    pool: &PgPool,
    products: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO products(name) SELECT UNNEST($1::text[]) ON CONFLICT DO NOTHING"#,
        products
    )
    .execute(pool)
    .await?;
    Ok(())
}
