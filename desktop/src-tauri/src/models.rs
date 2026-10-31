use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Product {
    pub id: i64,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category: String,
    pub brand: String,
    pub size: String,
    pub aliases: String,
    pub unit: i64,
    pub cost_cents: Option<i64>,
    pub price_cents: i64,
    pub wholesale_cents: i64,
    pub stock_units: i64,
    pub minimum_units: i64,
    pub tax_basis_points: i64,
    pub pack_name: String,
    pub pack_units: i64,
    pub active: bool,
}
impl Default for Product {
    fn default() -> Self {
        Self {
            id: 0,
            sku: String::new(),
            barcode: None,
            name: String::new(),
            category: "General".into(),
            brand: String::new(),
            size: String::new(),
            aliases: String::new(),
            unit: 0,
            cost_cents: None,
            price_cents: 0,
            wholesale_cents: 0,
            stock_units: 0,
            minimum_units: 0,
            tax_basis_points: 1600,
            pack_name: "Caja".into(),
            pack_units: 10000,
            active: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: i64,
    pub username: String,
    pub name: String,
    pub role: i64,
}
impl Actor {
    pub fn can(&self, permission: &str) -> bool {
        self.role == 0
            || ["sell", "quote", "cash"].contains(&permission)
            || self.role == 2
                && ["catalog", "stock", "purchase", "credit", "discount"].contains(&permission)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub store_name: String,
    pub admin_username: String,
    pub admin_name: String,
    pub admin_password: String,
    pub seller_username: String,
    pub seller_name: String,
    pub seller_password: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub tax_id: String,
    #[serde(default = "ticket_width")]
    pub ticket_width_mm: i64,
}
fn ticket_width() -> i64 {
    80
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleItem {
    pub product_id: i64,
    pub quantity_units: i64,
    #[serde(default)]
    pub wholesale: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub method: i64,
    pub amount_cents: i64,
    #[serde(default)]
    pub reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Checkout {
    pub request_id: String,
    pub items: Vec<SaleItem>,
    pub payments: Vec<Payment>,
    pub tendered_cents: i64,
    pub customer_id: Option<i64>,
    pub due_date: Option<String>,
    pub discount_parts: i64,
    pub quote_id: Option<i64>,
    pub held_sale_id: Option<i64>,
}
impl Default for Checkout {
    fn default() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            items: vec![],
            payments: vec![],
            tendered_cents: 0,
            customer_id: None,
            due_date: None,
            discount_parts: 0,
            quote_id: None,
            held_sale_id: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeLine {
    pub product_id: i64,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub unit: i64,
    pub quantity_units: i64,
    pub price_cents: i64,
    pub cost_cents: Option<i64>,
    pub tax_basis_points: i64,
    pub total_cents: i64,
    pub tax_cents: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Charge {
    pub lines: Vec<ChargeLine>,
    pub total_cents: i64,
    pub tax_cents: i64,
    pub discount_cents: i64,
}
