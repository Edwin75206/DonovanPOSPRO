use crate::{
    db,
    engine::Engine,
    error::{Result, ensure, text},
    models::Product,
    money,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use unicode_normalization::UnicodeNormalization;

pub fn product(db: &Connection, id: i64) -> Result<Product> {
    Ok(serde_json::from_value(db::one(
        db,
        "SELECT * FROM Product WHERE Id=?1",
        [id],
    )?)?)
}
pub fn validate(p: &mut Product) -> Result<()> {
    p.sku = text(&p.sku, "Clave", 40)?.to_uppercase();
    p.name = text(&p.name, "Descripción", 200)?;
    p.category = text(&p.category, "Categoría", 80)?;
    p.barcode = p
        .barcode
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| text(s, "Código de barras", 64))
        .transpose()?;
    ensure(
        p.brand.len() <= 200 && p.size.len() <= 200 && p.aliases.len() <= 1000,
        "VALIDATION",
        "La marca, medida o términos de búsqueda son demasiado largos.",
    )?;
    ensure(
        (0..=money::MAX_AMOUNT).contains(&p.price_cents)
            && (0..=money::MAX_AMOUNT).contains(&p.wholesale_cents)
            && p.cost_cents
                .is_none_or(|c| (0..=money::MAX_AMOUNT).contains(&c)),
        "VALIDATION",
        "El precio y costo deben ser positivos y estar dentro del límite permitido.",
    )?;
    ensure(
        (0..=money::MAX_UNITS).contains(&p.stock_units)
            && (0..=money::MAX_UNITS).contains(&p.minimum_units)
            && (0..=10000).contains(&p.tax_basis_points),
        "VALIDATION",
        "Revisa las existencias, el mínimo y el impuesto.",
    )?;
    money::quantity(p.unit, p.pack_units)?;
    if p.stock_units > 0 {
        money::quantity(p.unit, p.stock_units)?;
    }
    if p.minimum_units > 0 {
        money::quantity(p.unit, p.minimum_units)?;
    }
    Ok(())
}
pub fn stock(
    db: &Connection,
    now: &str,
    user: i64,
    id: i64,
    delta: i64,
    kind: &str,
    reference: &str,
    reason: &str,
) -> Result<()> {
    let p = product(db, id)?;
    let balance = p
        .stock_units
        .checked_add(delta)
        .filter(|n| *n >= 0 && *n <= money::MAX_UNITS);
    ensure(
        balance.is_some(),
        "INSUFFICIENT_STOCK",
        &format!(
            "No hay suficientes unidades de {}. Disponibles: {:.4}. Modifica la cantidad para continuar.",
            p.name,
            p.stock_units as f64 / 10000.0
        ),
    )?;
    db.execute(
        "UPDATE Product SET StockUnits=?1 WHERE Id=?2",
        params![balance, id],
    )?;
    db.execute("INSERT INTO StockMovement(ProductId,CreatedAt,UserId,Kind,Reference,Reason,DeltaUnits,BalanceUnits) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,now,user,kind,reference,reason,delta,balance])?;
    Ok(())
}
pub fn persist(db: &Connection, p: &Product) -> Result<i64> {
    let args = params![
        p.sku,
        p.barcode,
        p.name,
        p.category,
        p.unit,
        p.cost_cents,
        p.price_cents,
        p.wholesale_cents,
        p.minimum_units,
        p.tax_basis_points,
        p.pack_name,
        p.pack_units,
        p.active,
        p.brand,
        p.size,
        p.aliases,
        p.id
    ];
    if p.id == 0 {
        db.execute("INSERT INTO Product(Sku,Barcode,Name,Category,Unit,CostCents,PriceCents,WholesaleCents,MinimumUnits,TaxBasisPoints,PackName,PackUnits,Active,Brand,Size,Aliases,StockUnits) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",args)?;
        Ok(db.last_insert_rowid())
    } else {
        let existing = product(db, p.id)?;
        if p.unit != existing.unit {
            ensure(
                existing.stock_units == 0
                    && db::scalar(
                        db,
                        "SELECT (SELECT COUNT(*) FROM StockMovement WHERE ProductId=?1)+(SELECT COUNT(*) FROM QuoteLine WHERE ProductId=?1)+(SELECT COUNT(*) FROM HeldSaleLine WHERE ProductId=?1)",
                        [p.id],
                    )? == 0,
                "UNIT_HAS_HISTORY",
                "Este producto ya tiene existencias o historial. Crea otro producto para utilizar una unidad diferente.",
            )?;
        }
        db.execute("UPDATE Product SET Sku=?1,Barcode=?2,Name=?3,Category=?4,Unit=?5,CostCents=?6,PriceCents=?7,WholesaleCents=?8,MinimumUnits=?9,TaxBasisPoints=?10,PackName=?11,PackUnits=?12,Active=?13,Brand=?14,Size=?15,Aliases=?16 WHERE Id=?17",args)?;
        Ok(p.id)
    }
}
impl Engine {
    pub fn product_for_code(&self, code: &str) -> Result<Value> {
        self.actor("sell")?;
        let code = code.trim();
        ensure(
            !code.is_empty() && code.len() <= 200,
            "INVALID_CODE",
            "Escanea un código o escribe la clave completa.",
        )?;
        let matches = db::all(
            self.db()?,
            "SELECT Id FROM Product WHERE Barcode=?1 OR Sku=?1 COLLATE NOCASE LIMIT 2",
            [code],
        )?;
        ensure(
            !matches.is_empty(),
            "CODE_NOT_FOUND",
            "No encontramos ese código. Busca el nombre y selecciona el producto.",
        )?;
        ensure(
            matches.len() == 1,
            "AMBIGUOUS_CODE",
            "Este código coincide con más de un artículo. Selecciona el producto por nombre y pide al administrador revisar las claves.",
        )?;
        let value = self.get_product(matches[0]["id"].as_i64().unwrap_or(0))?;
        ensure(
            value["active"].as_bool() == Some(true),
            "PRODUCT_UNAVAILABLE",
            "El producto está archivado y no se puede vender.",
        )?;
        ensure(
            value["stockUnits"].as_i64().unwrap_or(0) > 0,
            "INSUFFICIENT_STOCK",
            "Producto agotado. No se agregó al ticket.",
        )?;
        Ok(value)
    }
    pub fn products(&self, search: &str, category: &str, status: &str, page: i64) -> Result<Value> {
        self.product_page(search, category, status, page, "name", false)
    }
    pub fn products_by_brand(
        &self,
        search: &str,
        category: &str,
        brand: &str,
        status: &str,
        page: i64,
    ) -> Result<Value> {
        self.filtered_product_page(search, category, brand, status, page, "name", false)
    }
    pub fn get_product(&self, id: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        let mut p = product(self.db()?, id)?;
        if !actor.can("catalog") {
            p.cost_cents = None;
        }
        Ok(serde_json::to_value(p)?)
    }
    pub fn product_page(
        &self,
        search: &str,
        category: &str,
        status: &str,
        page: i64,
        sort: &str,
        descending: bool,
    ) -> Result<Value> {
        self.filtered_product_page(search, category, "", status, page, sort, descending)
    }
    fn filtered_product_page(
        &self,
        search: &str,
        category: &str,
        brand: &str,
        status: &str,
        page: i64,
        sort: &str,
        descending: bool,
    ) -> Result<Value> {
        let actor = self.actor("sell")?;
        ensure(
            search.chars().count() <= 200
                && brand.chars().count() <= 200
                && page >= 0
                && page <= 100000,
            "VALIDATION",
            "Acorta la búsqueda o regresa a la primera página.",
        )?;
        ensure(
            ["active", "archived", "low", "empty"].contains(&status),
            "VALIDATION",
            "Selecciona un filtro válido de existencias.",
        )?;
        let order = match sort {
            "sku" => "p.Sku COLLATE NOCASE",
            "name" => "p.Name COLLATE NOCASE",
            "category" => "p.Category COLLATE NOCASE",
            "stockUnits" => "p.StockUnits",
            "priceCents" => "p.PriceCents",
            "wholesaleCents" => "p.WholesaleCents",
            "costCents" => {
                ensure(
                    actor.can("catalog"),
                    "FORBIDDEN",
                    "Tu cuenta no puede consultar los costos.",
                )?;
                "p.CostCents"
            }
            _ => {
                return Err(crate::error::AppError::rule(
                    "VALIDATION",
                    "Selecciona una columna válida para ordenar.",
                ));
            }
        };
        let direction = if descending { "DESC" } else { "ASC" };
        let normalized: String = search
            .nfkd()
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            .collect();
        let tokens: Vec<String> = normalized
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .take(12)
            .map(|s| format!("\"{s}\"*"))
            .collect();
        let query = tokens.join(" AND ");
        let where_sql = "WHERE (?1='' OR p.Id IN (SELECT rowid FROM ProductSearch WHERE ProductSearch MATCH ?1) OR p.Sku=?2 COLLATE NOCASE OR p.Barcode=?2) AND (?3='' OR p.Category=?3) AND ((?4='archived' AND p.Active=0) OR (?4<>'archived' AND p.Active=1)) AND (?4<>'low' OR p.StockUnits<=p.MinimumUnits) AND (?4<>'empty' OR p.StockUnits=0) AND (?5='' OR p.Brand=?5)";
        let total = db::scalar(
            self.db()?,
            &format!("SELECT COUNT(*) FROM Product p {where_sql}"),
            params![query, search.trim(), category, status, brand],
        )?;
        let mut items = db::all(
            self.db()?,
            &format!(
                "SELECT p.* FROM Product p {where_sql} ORDER BY CASE WHEN p.Sku=?2 COLLATE NOCASE OR p.Barcode=?2 THEN 0 ELSE 1 END,{order} {direction},p.Id LIMIT 60 OFFSET ?6"
            ),
            params![query, search.trim(), category, status, brand, page * 60],
        )?;
        if !actor.can("catalog") {
            for item in &mut items {
                item["costCents"] = Value::Null;
            }
        }
        let categories = db::all(
            self.db()?,
            "SELECT Category AS name,COUNT(*) AS count FROM Product WHERE Active=1 GROUP BY Category ORDER BY Category",
            [],
        )?;
        let brands = db::all(
            self.db()?,
            "SELECT Brand AS name,COUNT(*) AS count FROM Product WHERE Active=1 AND trim(Brand)<>'' GROUP BY Brand ORDER BY Brand",
            [],
        )?;
        Ok(
            json!({"items":items,"total":total,"page":page,"pageSize":60,"categories":categories,"brands":brands}),
        )
    }
    pub fn save_product(&mut self, mut p: Product) -> Result<Value> {
        let actor = self.actor("catalog")?;
        validate(&mut p)?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = persist(&tx, &p)?;
        if p.id == 0 && p.stock_units > 0 {
            stock(
                &tx,
                &now,
                actor.id,
                id,
                p.stock_units,
                "Alta",
                "Catálogo",
                "Existencia inicial",
            )?;
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Guardar producto",
            &id.to_string(),
            "",
        )?;
        let saved = db::one(&tx, "SELECT * FROM Product WHERE Id=?1", [id])?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn adjust_stock(&mut self, id: i64, delta: i64, reason: &str) -> Result<Value> {
        let actor = self.actor("stock")?;
        let reason = text(reason, "Motivo", 300)?;
        ensure(
            delta != 0,
            "VALIDATION",
            "La cantidad de ajuste no puede ser cero.",
        )?;
        money::quantity(
            product(self.db()?, id)?.unit,
            delta.checked_abs().unwrap_or(i64::MAX),
        )?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        stock(
            &tx,
            &now,
            actor.id,
            id,
            delta,
            "Ajuste",
            "Inventario",
            &reason,
        )?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Ajustar inventario",
            &id.to_string(),
            &reason,
        )?;
        let result = db::one(&tx, "SELECT * FROM Product WHERE Id=?1", [id])?;
        tx.commit()?;
        Ok(result)
    }
    pub fn stock_history(&self, id: i64) -> Result<Value> {
        self.actor("sell")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT * FROM StockMovement WHERE ProductId=?1 ORDER BY Id DESC LIMIT 500",
            [id]
        )?))
    }
}
