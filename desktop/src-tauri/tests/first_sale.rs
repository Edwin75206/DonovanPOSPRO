use donovan_core::{Engine,models::{Setup,Product,SaleItem,Payment,Checkout},db};
use tempfile::TempDir;

fn setup()->Setup{Setup{store_name:"Negocio de prueba".into(),admin_username:"admin".into(),admin_name:"Administración".into(),admin_password:"Solo-pruebas-2026".into(),seller_username:"vendedor".into(),seller_name:"Vendedor".into(),seller_password:"Venta-pruebas-2026".into(),address:"Calle de prueba".into(),phone:"".into(),tax_id:"".into(),ticket_width_mm:80}}
#[test]
fn first_full_sale_survives_restart_and_retries_without_double_stock(){
 let directory=TempDir::new().unwrap();
 let mut engine=Engine::open(directory.path()).unwrap();
 engine.setup(setup()).unwrap();engine.login("admin","Solo-pruebas-2026").unwrap();
 let product=engine.save_product(Product{sku:"CABLE-12".into(),name:"Cable calibre 12".into(),unit:1,price_cents:1850,cost_cents:Some(1000),stock_units:100000,..Default::default()}).unwrap();
 let id=product["id"].as_i64().unwrap();engine.cash_open(100000).unwrap();
 let request=Checkout{items:vec![SaleItem{product_id:id,quantity_units:25000,wholesale:false}],payments:vec![Payment{method:0,amount_cents:4625,reference:"".into()}],tendered_cents:5000,..Default::default()};
 let sale=engine.checkout(&request).unwrap();assert_eq!(sale["sale"]["totalCents"],4625);assert_eq!(sale["sale"]["changeCents"],375);
 let receipt_id=sale["sale"]["id"].as_i64().unwrap();assert_eq!(engine.checkout(&request).unwrap()["sale"]["id"],receipt_id);
 assert_eq!(db::scalar(engine.db().unwrap(),"SELECT StockUnits FROM Product WHERE Id=?1",[id]).unwrap(),75000);
 assert_eq!(db::scalar(engine.db().unwrap(),"SELECT COUNT(*) FROM Sale",[]).unwrap(),1);
 drop(engine);
 let engine=Engine::open(directory.path()).unwrap();assert_eq!(engine.actor("sell").unwrap().username,"admin");
 assert_eq!(engine.sale(receipt_id).unwrap()["sale"]["totalCents"],4625);
 assert_eq!(engine.cash_overview().unwrap()["expectedCents"],104625);
 db::integrity(engine.db().unwrap()).unwrap();
}
