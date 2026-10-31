use crate::{
    engine::Engine,
    error::{Result, ensure},
    models::{Checkout, Payment, Product, SaleItem, Setup},
};
use serde_json::Value;
impl Engine {
    pub fn seed_demo(&mut self) -> Result<()> {
        ensure(
            self.bootstrap()?["needsSetup"] == true,
            "DEMO_SEPARATE",
            "La demostración requiere una carpeta nueva y separada.",
        )?;
        self.demo = true;
        let password = uuid::Uuid::new_v4().to_string();
        let profile: Value = serde_json::from_str(include_str!(
            "../../../assets/branding/customer-profile.json"
        ))?;
        self.setup(Setup {
            store_name: profile["storeName"].as_str().unwrap_or("Mi negocio").into(),
            admin_username: "administrador".into(),
            admin_name: "Administración · DEMO".into(),
            admin_password: password.clone(),
            seller_username: "vendedor".into(),
            seller_name: "Negociante Uno · DEMO".into(),
            seller_password: password.clone(),
            address: "Datos de demostración · sin información de clientes".into(),
            phone: "".into(),
            tax_id: "".into(),
            ticket_width_mm: 80,
        })?;
        self.login("administrador", &password)?;
        self.demo_password = Some(password);
        let samples = [
            (
                "CEM-50",
                "Cemento gris CPC 30R",
                "Construcción",
                "CEMEX",
                0,
                23500,
                500000,
                "Saco 50 kg",
            ),
            (
                "MOR-50",
                "Mortero para construcción",
                "Construcción",
                "CEMEX",
                0,
                16900,
                400000,
                "Saco 50 kg",
            ),
            (
                "VAR-38",
                "Varilla corrugada 3/8\"",
                "Construcción",
                "Deacero",
                0,
                18900,
                780000,
                "12 metros",
            ),
            (
                "BLO-12",
                "Block de concreto 12 × 20 × 40",
                "Construcción",
                "",
                0,
                1850,
                12800000,
                "12 cm",
            ),
            (
                "MAR-16",
                "Martillo de uña 16 oz",
                "Herramientas",
                "Truper",
                0,
                18900,
                180000,
                "Mango de fibra",
            ),
            (
                "TAL-12",
                "Taladro percutor 1/2\"",
                "Herramientas",
                "Truper",
                0,
                149900,
                60000,
                "650 W",
            ),
            (
                "DES-6",
                "Juego de desarmadores",
                "Herramientas",
                "Pretul",
                0,
                24500,
                240000,
                "6 piezas",
            ),
            (
                "BRO-2",
                "Brocha profesional 2\"",
                "Pintura",
                "Truper",
                0,
                4800,
                350000,
                "Cerda natural",
            ),
            (
                "PIN-19",
                "Pintura vinílica blanco mate",
                "Pintura",
                "Comex",
                0,
                129900,
                80000,
                "Cubeta 19 L",
            ),
            (
                "CAB-12",
                "Cable THW calibre 12",
                "Electricidad",
                "Condumex",
                1,
                1850,
                4500000,
                "Rojo · por metro",
            ),
            (
                "CIN-18",
                "Cinta aislante 18 m",
                "Electricidad",
                "Truper",
                0,
                2400,
                50000,
                "Negra",
            ),
            (
                "TUB-12",
                "Tubo PVC hidráulico 1/2\"",
                "Plomería",
                "Amanco",
                0,
                7800,
                700000,
                "6 metros",
            ),
            (
                "COD-12",
                "Codo PVC hidráulico 90°",
                "Plomería",
                "Amanco",
                0,
                850,
                1200000,
                "1/2 pulgada",
            ),
            (
                "GUA-1",
                "Guantes de trabajo reforzados",
                "Seguridad",
                "Truper",
                0,
                8900,
                30000,
                "Talla grande",
            ),
            (
                "ARE-1",
                "Arena para construcción",
                "Construcción",
                "",
                2,
                450,
                8500000,
                "Por kilogramo",
            ),
            (
                "LLA-12",
                "Llave de jardín 1/2\"",
                "Plomería",
                "Foset",
                0,
                11500,
                20000,
                "Latón",
            ),
        ];
        let mut ids = vec![];
        for (index, (sku, name, category, brand, unit, price, stock, size)) in
            samples.iter().enumerate()
        {
            let p = self.save_product(Product {
                sku: sku.to_string(),
                barcode: Some(format!("75000000{:05}", index + 1)),
                name: name.to_string(),
                category: category.to_string(),
                brand: brand.to_string(),
                size: size.to_string(),
                unit: *unit,
                price_cents: *price,
                wholesale_cents: price * 90 / 100,
                cost_cents: Some(price * 68 / 100),
                stock_units: *stock,
                minimum_units: 50000,
                ..Default::default()
            })?;
            ids.push(p["id"].as_i64().unwrap_or(0));
        }
        self.cash_open(100000)?;
        for i in 0..18 {
            let index = [0usize, 4, 6, 7, 9, 11][i % 6];
            let price = samples[index].5;
            let sale = self.checkout(&Checkout {
                items: vec![SaleItem {
                    product_id: ids[index],
                    quantity_units: 10000,
                    wholesale: false,
                }],
                payments: vec![Payment {
                    method: if i % 3 == 0 { 1 } else { 0 },
                    amount_cents: price,
                    reference: "Demostración".into(),
                }],
                tendered_cents: if i % 3 == 0 { 0 } else { price },
                ..Default::default()
            })?;
            let clock_text = self.now_string();
            let seconds_today = clock_text[11..13].parse::<i64>().unwrap_or(0) * 3600
                + clock_text[14..16].parse::<i64>().unwrap_or(0) * 60
                + clock_text[17..19].parse::<i64>().unwrap_or(0);
            let time = crate::engine::stamp(
                self.now()
                    - (5 - i / 3) as i64 * 86400
                    - (2 - i % 3) as i64 * 300.min(seconds_today / 2),
            );
            self.db()?.execute(
                "UPDATE Sale SET CreatedAt=?1 WHERE Id=?2",
                rusqlite::params![time, sale["sale"]["id"].as_i64()],
            )?;
            let folio = sale["sale"]["folio"].as_str().unwrap_or("");
            self.db()?.execute(
                "UPDATE CashMovement SET CreatedAt=?1 WHERE Reference=?2",
                rusqlite::params![time, folio],
            )?;
            self.db()?.execute(
                "UPDATE StockMovement SET CreatedAt=?1 WHERE Reference=?2",
                rusqlite::params![time, folio],
            )?;
        }
        self.db()?.execute("UPDATE CashSession SET OpenedAt=(SELECT MIN(CreatedAt) FROM Sale) WHERE ClosedAt IS NULL",[])?;
        Ok(())
    }
    pub fn demo_login(&mut self, role: i64) -> Result<Value> {
        ensure(
            self.demo,
            "FORBIDDEN",
            "Esta función solo está disponible en demostración.",
        )?;
        let password = self.demo_password.clone().unwrap_or_default();
        self.login(
            if role == 0 {
                "administrador"
            } else {
                "vendedor"
            },
            &password,
        )
    }
}
