use serde_derive::{Serialize, Deserialize};


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub status: String,
    pub created_date_time: String,
    pub last_updated_date_time: String,
    pub analyze_result: AnalyzeResult,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResult {
    pub api_version: String,
    pub model_id: String,
    pub string_index_type: String,
    pub content: String,
    pub pages: Vec<Page>,
    pub styles: Vec<Style>,
    pub documents: Vec<Document>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub page_number: i64,
    pub angle: i64,
    pub width: i64,
    pub height: i64,
    pub unit: String,
    pub words: Vec<Word>,
    pub lines: Vec<Line>,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub content: String,
    pub polygon: Vec<i64>,
    pub confidence: f64,
    pub span: Span,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub offset: i64,
    pub length: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub content: String,
    pub polygon: Vec<i64>,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    pub confidence: f64,
    pub spans: Vec<Span>,
    pub is_handwritten: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub doc_type: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub fields: Fields,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingRegion {
    pub page_number: i64,
    pub polygon: Vec<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    #[serde(rename = "Items")]
    pub items: Items,
    #[serde(rename = "MerchantName")]
    pub merchant_name: MerchantName,
    #[serde(rename = "TaxDetails")]
    pub tax_details: TaxDetails,
    #[serde(rename = "Total")]
    pub total: Total,
    #[serde(rename = "TotalTax")]
    pub total_tax: TotalTax,
    #[serde(rename = "TransactionDate")]
    pub transaction_date: TransactionDate,
    #[serde(rename = "TransactionTime")]
    pub transaction_time: TransactionTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_array: Vec<ValueArray>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueArray {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_object: ValueObject,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueObject {
    #[serde(rename = "Description")]
    pub description: Description,
    #[serde(rename = "TotalPrice")]
    pub total_price: TotalPrice,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Description {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_string: String,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalPrice {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_number: f64,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantName {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_string: String,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxDetails {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_array: Vec<ValueArray2>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueArray2 {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_object: ValueObject2,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueObject2 {
    #[serde(rename = "Amount")]
    pub amount: Amount,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_currency: ValueCurrency,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueCurrency {
    pub amount: f64,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Total {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_number: f64,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalTax {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_number: f64,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDate {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_date: String,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionTime {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value_time: String,
    pub content: String,
    pub bounding_regions: Vec<BoundingRegion>,
    pub confidence: f64,
    pub spans: Vec<Span>,
}