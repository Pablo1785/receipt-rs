use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiVersion(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentModelId(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub enum StringIndexType {
    #[serde(rename = "textElements")]
    TextElements,
    #[serde(rename = "unicodeCodePoint")]
    UnicodeCodePoint,
    #[serde(rename = "utf16CodeUnit")]
    Utf16CodeUnit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentSpan {
    pub offset: i32,
    pub length: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Confidence(f64);

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentWord {
    pub content: String,
    pub polygon: Option<BoundingPolygon>,
    pub span: DocumentSpan,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SelectionMarkState {
    Selected,
    Unselected,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentSelectionMarkState {
    #[serde(rename = "SelectionMarkState")]
    pub state: SelectionMarkState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentSelectionMark {
    pub state: DocumentSelectionMarkState,
    pub polygon: Option<BoundingPolygon>,
    pub span: DocumentSpan,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundingPolygon(Vec<i64>);

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentLine {
    pub content: String,
    pub polygon: Option<BoundingPolygon>,
    pub spans: Vec<DocumentSpan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentAnnotationKind {
    Check,
    Cross,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentAnnotation {
    pub kind: DocumentAnnotationKind,
    pub polygon: Option<BoundingPolygon>,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentBarcodeKind {
    QRCode,
    PDF417,
    UPCA,
    UPCE,
    Code39,
    Code128,
    EAN8,
    EAN13,
    DataBar,
    Code93,
    Codabar,
    DataBarExpanded,
    ITF,
    MicroQRCode,
    Aztec,
    DataMatrix,
    MaxiCode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentBarcode {
    pub kind: DocumentBarcodeKind,
    pub value: String,
    pub polygon: Option<BoundingPolygon>,
    pub span: DocumentSpan,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentFormulaKind {
    Inline,
    Display,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentFormula {
    pub kind: DocumentFormulaKind,
    pub value: String,
    pub polygon: Option<BoundingPolygon>,
    pub span: DocumentSpan,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LengthUnit {
    #[serde(rename = "pixel")]
    Pixel,
    #[serde(rename = "inch")]
    Inch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentPage {
    pub pageNumber: i32,
    pub spans: Vec<DocumentSpan>,
    pub angle: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub unit: Option<LengthUnit>,
    pub words: Option<Vec<DocumentWord>>,
    pub selectionMarks: Option<Vec<DocumentSelectionMark>>,
    pub lines: Option<Vec<DocumentLine>>,
    pub annotations: Option<Vec<DocumentAnnotation>>,
    pub barcodes: Option<Vec<DocumentBarcode>>,
    pub formulas: Option<Vec<DocumentFormula>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ParagraphRole {
    PageHeader,
    PageFooter,
    PageNumber,
    Title,
    SectionHeading,
    Footnote,
    FormulaBlock,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentParagraph {
    pub role: Option<ParagraphRole>,
    pub content: String,
    pub boundingRegions: Option<Vec<BoundingRegion>>,
    pub spans: Vec<DocumentSpan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentTableCellKind {
    Content,
    RowHeader,
    ColumnHeader,
    StubHead,
    Description,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentTableCell {
    pub kind: Option<DocumentTableCellKind>,
    pub rowIndex: i32,
    pub columnIndex: i32,
    pub rowSpan: Option<i32>,
    pub columnSpan: Option<i32>,
    pub content: String,
    pub boundingRegions: Option<Vec<BoundingRegion>>,
    pub spans: Vec<DocumentSpan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentTable {
    pub rowCount: i32,
    pub columnCount: i32,
    pub cells: Vec<DocumentTableCell>,
    pub boundingRegions: Option<Vec<BoundingRegion>>,
    pub spans: Vec<DocumentSpan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentKeyValueElement {
    pub content: String,
    pub boundingRegions: Option<Vec<BoundingRegion>>,
    pub spans: Vec<DocumentSpan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentKeyValuePair {
    pub key: DocumentKeyValueElement,
    pub value: Option<DocumentKeyValueElement>,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentStyle {
    pub isHandwritten: Option<bool>,
    pub similarFontFamily: Option<String>,
    pub fontStyle: Option<FontStyle>,
    pub fontWeight: Option<FontWeight>,
    pub color: Option<String>,
    pub backgroundColor: Option<String>,
    pub spans: Vec<DocumentSpan>,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentLanguage {
    pub locale: String,
    pub spans: Vec<DocumentSpan>,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocType(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub docType: DocType,
    pub boundingRegions: Vec<BoundingRegion>,
    pub spans: Vec<DocumentSpan>,
    pub fields: Receipt,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResult {
    pub apiVersion: ApiVersion,
    pub modelId: DocumentModelId,
    pub stringIndexType: StringIndexType,
    pub content: String,
    pub pages: Vec<DocumentPage>,
    pub paragraphs: Option<Vec<DocumentParagraph>>,
    pub tables: Option<Vec<DocumentTable>>,
    pub keyValuePairs: Option<Vec<DocumentKeyValuePair>>,
    pub styles: Option<Vec<DocumentStyle>>,
    pub languages: Option<Vec<DocumentLanguage>>,
    pub documents: Option<Vec<Document>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResultOperation {
    pub status: String,
    pub createdDateTime: String,
    pub lastUpdatedDateTime: String,
    pub error: Option<serde_json::Value>, // Represents a dynamic JSON structure for Error type
    pub analyzeResult: Option<AnalyzeResult>, // Represents a dynamic JSON structure for AnalyzeResult type
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentField {
    #[serde(rename = "type")]
    pub field_type: DocumentFieldType,
    pub value_string: Option<String>,
    pub value_date: Option<String>, // Should be parsed to a DateTime type in Rust
    pub value_time: Option<String>, // Should be parsed to a DateTime type in Rust
    pub value_phone_number: Option<String>,
    pub value_number: Option<f64>,
    pub value_integer: Option<i64>,
    pub value_selection_mark: Option<DocumentSelectionMarkState>,
    pub value_signature: Option<DocumentSignatureType>,
    pub value_country_region: Option<String>,
    pub value_array: Option<Vec<DocumentField>>,
    pub value_object: Option<HashMap<String, DocumentField>>,
    pub value_currency: Option<CurrencyValue>,
    pub value_address: Option<AddressValue>,
    pub value_boolean: Option<bool>,
    pub content: Option<String>,
    pub bounding_regions: Option<Vec<BoundingRegion>>,
    pub spans: Option<Vec<DocumentSpan>>,
    pub confidence: Option<Confidence>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentFieldType {
    #[serde(rename = "type")]
    pub field_type: FieldType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FieldType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "phoneNumber")]
    PhoneNumber,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "selectionMark")]
    SelectionMark,
    #[serde(rename = "countryRegion")]
    CountryRegion,
    #[serde(rename = "signature")]
    Signature,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "currency")]
    Currency,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentSignatureType {
    #[serde(rename = "signed")]
    Signed,
    #[serde(rename = "unsigned")]
    Unsigned,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrencyValue {
    pub amount: f64,
    pub currencySymbol: Option<String>,
    pub currencyCode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressValue {
    pub houseNumber: Option<String>,
    pub poBox: Option<String>,
    pub road: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postalCode: Option<String>,
    pub countryRegion: Option<String>,
    pub streetAddress: Option<String>,
    pub unit: Option<String>,
    pub cityDistrict: Option<String>,
    pub stateDistrict: Option<String>,
    pub suburb: Option<String>,
    pub house: Option<String>,
    pub level: Option<String>,
}

// TODO: Hotel Receipts have different Items and additional fields

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
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
    #[serde(rename = "Quantity")]
    pub quantity: Option<f64>,
    #[serde(rename = "Price")]
    pub unit_price: Option<f64>,
    #[serde(rename = "ProductCode")]
    pub product_code: Option<String>,
    #[serde(rename = "QuantityUnit")]
    pub quantity_unit: Option<String>,
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
pub struct Span {
    pub offset: i64,
    pub length: i64,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingRegion {
    pub page_number: i64,
    pub polygon: Vec<i64>,
}

