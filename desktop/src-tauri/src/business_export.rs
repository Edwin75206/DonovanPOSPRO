//! Read-only Excel exports. Each workbook is built from one consistent snapshot.
use crate::{
    Engine, catalog,
    database::Connection,
    documents::layout::unit,
    error::{AppError, Result},
    exporting::{excel, save},
};
use chrono::NaiveDate;
use rusqlite::{params, types::Value as SqlValue};
use rust_xlsxwriter::{Color, ExcelDateTime, Format, Workbook, Worksheet};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BusinessExport {
    SalesToday,
    Sales,
    Customers,
    Suppliers,
    Inventory,
    Purchases,
    Quotes,
    Returns,
    Cash,
    Credit,
    All,
}

#[derive(Clone, Copy)]
enum Cell {
    Text,
    Id,
    Money,
    Quantity,
    Minimum,
    Percent,
    Date,
    Unit,
    Payment,
    Active,
    YesNo,
}
use Cell::*;

type Column = (&'static str, &'static str, Cell);
struct Sheet {
    name: &'static str,
    from: &'static str,
    columns: &'static [Column],
}

const SALES: Sheet = Sheet {
    name: "Ventas",
    from: "Sale r JOIN CashSession c ON c.Id=r.CashSessionId",
    columns: &[
        ("r.Folio", "Folio", Text),
        ("r.CreatedAt", "Fecha de venta", Date),
        ("r.CustomerName", "Cliente", Text),
        ("r.Cashier", "Atendió", Text),
        ("c.Folio", "Folio de caja", Text),
        (
            "CASE WHEN r.Cancelled=1 THEN 'Cancelada' WHEN EXISTS(SELECT 1 FROM SaleReturn d WHERE d.SaleId=r.Id) THEN 'Con devolución' ELSE 'Registrada' END",
            "Estado",
            Text,
        ),
        ("r.TotalCents-r.TaxCents", "Subtotal sin IVA", Money),
        ("r.TaxCents", "IVA incluido", Money),
        ("r.DiscountCents", "Descuento aplicado", Money),
        ("r.TotalCents", "Importe original", Money),
        (
            "COALESCE((SELECT SUM(d.TotalCents) FROM SaleReturn d WHERE d.SaleId=r.Id),0)",
            "Devuelto acumulado",
            Money,
        ),
        ("r.TenderedCents", "Efectivo recibido", Money),
        ("r.ChangeCents", "Cambio", Money),
        ("r.CustomerId", "ID cliente", Id),
        ("r.Id", "ID venta", Id),
    ],
};
const SALE_LINES: Sheet = Sheet {
    name: "Partidas de ventas",
    from: "SaleLine r JOIN Sale s ON s.Id=r.SaleId",
    columns: &[
        ("s.Folio", "Folio de venta", Text),
        ("s.CreatedAt", "Fecha de venta", Date),
        ("r.Sku", "Clave del producto", Text),
        ("r.Name", "Producto", Text),
        ("r.Category", "Categoría", Text),
        ("r.Unit", "Unidad", Unit),
        ("r.QuantityUnits", "Cantidad", Quantity),
        ("r.PriceCents", "Precio unitario", Money),
        ("r.TaxCents", "IVA incluido", Money),
        ("r.TotalCents", "Importe", Money),
        ("r.ReturnedUnits", "Cantidad devuelta", Quantity),
        ("r.RefundedCents", "Importe devuelto", Money),
        ("s.Id", "ID venta", Id),
        ("r.ProductId", "ID producto", Id),
    ],
};
const SALE_PAYMENTS: Sheet = Sheet {
    name: "Pagos de ventas",
    from: "PaymentRecord r JOIN Sale s ON s.Id=r.SaleId",
    columns: &[
        ("s.Folio", "Folio de venta", Text),
        ("s.CreatedAt", "Fecha de venta", Date),
        ("r.Method", "Forma de pago", Payment),
        ("r.AmountCents", "Importe", Money),
        ("r.Reference", "Referencia", Text),
        ("s.Id", "ID venta", Id),
    ],
};
const CUSTOMERS: Sheet = Sheet {
    name: "Clientes",
    from: "Customer r",
    columns: &[
        ("r.Name", "Cliente", Text),
        ("r.Phone", "Teléfono", Text),
        ("r.Email", "Correo electrónico", Text),
        ("r.Address", "Domicilio", Text),
        ("r.CreditLimitCents", "Límite de crédito", Money),
        (
            "COALESCE((SELECT SUM(c.AmountCents) FROM CreditEntry c WHERE c.CustomerId=r.Id),0)",
            "Saldo pendiente",
            Money,
        ),
        ("r.Active", "Estado", Active),
        ("r.Id", "ID cliente", Id),
    ],
};
const SUPPLIERS: Sheet = Sheet {
    name: "Proveedores",
    from: "Supplier r",
    columns: &[
        ("r.Name", "Proveedor", Text),
        ("r.Phone", "Teléfono", Text),
        ("r.Email", "Correo electrónico", Text),
        ("r.Address", "Domicilio", Text),
        ("r.Active", "Estado", Active),
        ("r.Id", "ID proveedor", Id),
    ],
};
const PRODUCTS: Sheet = Sheet {
    name: "Productos",
    from: "Product r LEFT JOIN AppMeta w ON w.Key=CONCAT('product-wholesale-minimum:',r.Id)",
    columns: &[
        ("r.Sku", "Clave interna", Text),
        ("r.Barcode", "Código de barras", Text),
        ("r.Name", "Producto", Text),
        ("r.Category", "Categoría", Text),
        ("r.Brand", "Marca", Text),
        ("r.Size", "Presentación comercial", Text),
        ("r.Unit", "Unidad base", Unit),
        ("r.StockUnits", "Existencia actual", Quantity),
        ("r.MinimumUnits", "Existencia mínima", Quantity),
        ("r.CostCents", "Costo base sin IVA", Money),
        ("r.PriceCents", "Precio público con IVA", Money),
        ("r.WholesaleCents", "Precio de mayoreo con IVA", Money),
        ("w.Value", "Mayoreo desde", Minimum),
        ("r.TaxBasisPoints", "IVA", Percent),
        ("r.Active", "Estado", Active),
        ("r.PackName", "Presentación de compra", Text),
        ("r.PackUnits", "Unidades por presentación", Quantity),
        ("r.Aliases", "Otros nombres", Text),
        ("r.Id", "ID producto", Id),
    ],
};
const STOCK: Sheet = Sheet {
    name: "Movimientos de inventario",
    from: "StockMovement r JOIN Product p ON p.Id=r.ProductId JOIN User u ON u.Id=r.UserId",
    columns: &[
        ("r.CreatedAt", "Fecha", Date),
        ("p.Sku", "Clave del producto", Text),
        ("p.Name", "Producto", Text),
        ("r.Kind", "Movimiento", Text),
        ("r.Reference", "Referencia", Text),
        ("p.Unit", "Unidad", Unit),
        ("r.DeltaUnits", "Cambio de existencia", Quantity),
        ("r.BalanceUnits", "Existencia resultante", Quantity),
        ("r.Reason", "Motivo", Text),
        ("u.Name", "Registró", Text),
        ("p.Id", "ID producto", Id),
    ],
};
const PURCHASES: Sheet = Sheet {
    name: "Compras",
    from: "Purchase r JOIN Supplier s ON s.Id=r.SupplierId JOIN User u ON u.Id=r.UserId
           LEFT JOIN PurchaseDetails d ON d.PurchaseId=r.Id",
    columns: &[
        ("r.Folio", "Folio de compra", Text),
        ("r.CreatedAt", "Fecha de compra", Date),
        ("s.Name", "Proveedor", Text),
        ("r.SupplierReference", "Referencia del proveedor", Text),
        ("u.Name", "Registró", Text),
        ("r.TotalCents", "Importe de compra", Money),
        ("d.PaidFromCash", "Pagada desde caja", YesNo),
        ("s.Id", "ID proveedor", Id),
        ("r.Id", "ID compra", Id),
    ],
};
const PURCHASE_LINES: Sheet = Sheet {
    name: "Partidas de compras",
    from: "PurchaseLine r JOIN Purchase c ON c.Id=r.PurchaseId JOIN Product p ON p.Id=r.ProductId
           LEFT JOIN PurchaseLineDetails d ON d.PurchaseLineId=r.Id",
    columns: &[
        ("c.Folio", "Folio de compra", Text),
        ("COALESCE(d.ProductSku,p.Sku)", "Clave del producto", Text),
        ("COALESCE(d.ProductName,p.Name)", "Producto", Text),
        ("COALESCE(d.Unit,p.Unit)", "Unidad base", Unit),
        ("r.QuantityUnits", "Cantidad en unidad base", Quantity),
        ("r.CostCents", "Costo unitario base", Money),
        ("d.ByPack", "Capturada por presentación", YesNo),
        ("d.PackName", "Presentación", Text),
        ("d.EnteredQuantityUnits", "Cantidad capturada", Quantity),
        ("d.EnteredCostCents", "Costo unitario capturado", Money),
        ("d.TotalCents", "Importe de la partida", Money),
        ("c.Id", "ID compra", Id),
        ("p.Id", "ID producto", Id),
    ],
};
const QUOTES: Sheet = Sheet {
    name: "Cotizaciones",
    from: "Quote r JOIN User u ON u.Id=r.UserId LEFT JOIN Sale s ON s.Id=r.SaleId",
    columns: &[
        ("r.Folio", "Folio de cotización", Text),
        ("r.CreatedAt", "Fecha de cotización", Date),
        ("r.ValidUntil", "Vigente hasta", Date),
        ("r.CustomerName", "Cliente", Text),
        ("u.Name", "Atendió", Text),
        ("r.Notes", "Notas", Text),
        ("r.TaxCents", "IVA incluido", Money),
        ("r.TotalCents", "Total cotizado", Money),
        ("s.Folio", "Folio de venta", Text),
        ("r.CustomerId", "ID cliente", Id),
        ("r.Id", "ID cotización", Id),
        ("r.SaleId", "ID venta", Id),
    ],
};
const QUOTE_LINES: Sheet = Sheet {
    name: "Partidas de cotizaciones",
    from: "QuoteLine r JOIN Quote q ON q.Id=r.QuoteId JOIN Product p ON p.Id=r.ProductId",
    columns: &[
        ("q.Folio", "Folio de cotización", Text),
        ("p.Sku", "Clave del producto", Text),
        ("r.Name", "Producto", Text),
        ("r.Unit", "Unidad", Unit),
        ("r.QuantityUnits", "Cantidad", Quantity),
        ("r.PriceCents", "Precio unitario", Money),
        ("r.TaxBasisPoints", "IVA", Percent),
        ("r.TotalCents", "Importe cotizado", Money),
        ("q.Id", "ID cotización", Id),
        ("p.Id", "ID producto", Id),
    ],
};
const RETURNS: Sheet = Sheet {
    name: "Devoluciones",
    from: "SaleReturn r JOIN Sale s ON s.Id=r.SaleId JOIN User u ON u.Id=r.UserId",
    columns: &[
        ("r.Folio", "Folio de devolución", Text),
        ("r.CreatedAt", "Fecha de devolución", Date),
        ("s.Folio", "Folio de venta", Text),
        ("s.CustomerName", "Cliente", Text),
        ("r.Reason", "Motivo", Text),
        ("r.TotalCents", "Total devuelto", Money),
        ("r.CreditReductionCents", "Reducción de crédito", Money),
        (
            "r.TotalCents-r.CreditReductionCents",
            "Importe reembolsado",
            Money,
        ),
        (
            "CASE WHEN r.TotalCents>r.CreditReductionCents THEN r.RefundMethod ELSE NULL END",
            "Forma de reembolso",
            Payment,
        ),
        ("u.Name", "Registró", Text),
        ("s.Id", "ID venta", Id),
        ("r.Id", "ID devolución", Id),
    ],
};
const RETURN_LINES: Sheet = Sheet {
    name: "Partidas de devoluciones",
    from: "ReturnLine r JOIN SaleReturn d ON d.Id=r.SaleReturnId JOIN SaleLine l ON l.Id=r.SaleLineId",
    columns: &[
        ("d.Folio", "Folio de devolución", Text),
        ("l.Sku", "Clave del producto", Text),
        ("l.Name", "Producto", Text),
        ("l.Unit", "Unidad", Unit),
        ("r.QuantityUnits", "Cantidad devuelta", Quantity),
        ("r.AmountCents", "Importe devuelto", Money),
        ("d.Id", "ID devolución", Id),
        ("l.ProductId", "ID producto", Id),
    ],
};
const CASH: Sheet = Sheet {
    name: "Caja",
    from: "CashSession r JOIN User u ON u.Id=r.UserId",
    columns: &[
        ("r.Folio", "Folio de caja", Text),
        ("r.OpenedAt", "Apertura", Date),
        ("r.ClosedAt", "Cierre", Date),
        ("u.Name", "Abrió la caja", Text),
        ("r.OpeningCents", "Fondo inicial", Money),
        (
            "COALESCE(r.ExpectedCents,r.OpeningCents+(SELECT COALESCE(SUM(m.AmountCents),0) FROM CashMovement m WHERE m.CashSessionId=r.Id))",
            "Efectivo esperado",
            Money,
        ),
        ("r.CountedCents", "Efectivo contado", Money),
        (
            "r.CountedCents-r.ExpectedCents",
            "Diferencia del cierre",
            Money,
        ),
        ("r.Notes", "Notas", Text),
    ],
};
const CASH_MOVEMENTS: Sheet = Sheet {
    name: "Movimientos de caja",
    from: "CashMovement r JOIN CashSession c ON c.Id=r.CashSessionId JOIN User u ON u.Id=r.UserId",
    columns: &[
        ("c.Folio", "Folio de caja", Text),
        ("r.CreatedAt", "Fecha", Date),
        ("r.Kind", "Movimiento", Text),
        ("r.Reference", "Referencia", Text),
        ("r.AmountCents", "Importe", Money),
        ("r.Reason", "Motivo", Text),
        ("u.Name", "Registró", Text),
    ],
};
const CREDIT: Sheet = Sheet {
    name: "Movimientos de crédito",
    from: "CreditEntry r JOIN Customer c ON c.Id=r.CustomerId JOIN Sale s ON s.Id=r.SaleId JOIN User u ON u.Id=r.UserId",
    columns: &[
        ("r.CreatedAt", "Fecha", Date),
        ("c.Name", "Cliente", Text),
        ("s.Folio", "Folio de venta", Text),
        ("r.Kind", "Movimiento", Text),
        ("r.AmountCents", "Importe", Money),
        ("r.DueDate", "Vencimiento", Date),
        ("r.Method", "Forma de pago", Payment),
        ("r.Reference", "Referencia", Text),
        ("u.Name", "Registró", Text),
        ("c.Id", "ID cliente", Id),
        ("s.Id", "ID venta", Id),
    ],
};

impl BusinessExport {
    fn sheets(self) -> (&'static str, &'static [Sheet]) {
        match self {
            Self::SalesToday => ("Ventas-del-dia", &[SALES, SALE_LINES, SALE_PAYMENTS]),
            Self::Sales => ("Ventas", &[SALES, SALE_LINES, SALE_PAYMENTS]),
            Self::Customers => ("Clientes", &[CUSTOMERS]),
            Self::Suppliers => ("Proveedores", &[SUPPLIERS]),
            Self::Inventory => ("Productos-e-inventario", &[PRODUCTS, STOCK]),
            Self::Purchases => ("Compras", &[PURCHASES, PURCHASE_LINES]),
            Self::Quotes => ("Cotizaciones", &[QUOTES, QUOTE_LINES]),
            Self::Returns => ("Devoluciones", &[RETURNS, RETURN_LINES]),
            Self::Cash => ("Caja", &[CASH, CASH_MOVEMENTS]),
            Self::Credit => ("Credito", &[CREDIT]),
            Self::All => (
                "Base-del-negocio",
                &[
                    SALES,
                    SALE_LINES,
                    SALE_PAYMENTS,
                    CUSTOMERS,
                    SUPPLIERS,
                    PRODUCTS,
                    STOCK,
                    PURCHASES,
                    PURCHASE_LINES,
                    QUOTES,
                    QUOTE_LINES,
                    RETURNS,
                    RETURN_LINES,
                    CASH,
                    CASH_MOVEMENTS,
                    CREDIT,
                ],
            ),
        }
    }
}

struct Formats {
    heading: Format,
    text: Format,
    money: Format,
    quantity: Format,
    whole_quantity: Format,
    percent: Format,
    date: Format,
    date_time: Format,
}
impl Formats {
    fn new() -> Self {
        Self {
            heading: Format::new()
                .set_bold()
                .set_font_color(Color::White)
                .set_background_color(Color::RGB(0x24433B))
                .set_text_wrap(),
            text: Format::new().set_num_format("@").set_text_wrap(),
            money: Format::new().set_num_format("\"$\"#,##0.00;-\"$\"#,##0.00"),
            quantity: Format::new().set_num_format("#,##0.####"),
            whole_quantity: Format::new().set_num_format("#,##0"),
            percent: Format::new().set_num_format("0.00%"),
            date: Format::new().set_num_format("dd/mm/yyyy"),
            date_time: Format::new().set_num_format("dd/mm/yyyy hh:mm:ss"),
        }
    }

    fn quantity(&self, scaled: i64) -> &Format {
        if scaled % 10_000 == 0 {
            &self.whole_quantity
        } else {
            &self.quantity
        }
    }
}

fn start_sheet<'a>(
    book: &'a mut Workbook,
    spec: &Sheet,
    index: u32,
    formats: &Formats,
) -> Result<&'a mut Worksheet> {
    let sheet = book.add_worksheet_with_constant_memory();
    let name = if index == 1 {
        spec.name.to_owned()
    } else {
        format!("{} {index}", spec.name)
    };
    sheet.set_name(name).map_err(excel)?;
    sheet.set_freeze_panes(1, 0).map_err(excel)?;
    sheet.set_row_height(0, 30).map_err(excel)?;
    for (index, &(_, label, kind)) in spec.columns.iter().enumerate() {
        let col = index as u16;
        let width = match kind {
            Text => 30.0,
            Date => 24.0,
            _ => 18.0,
        };
        sheet.set_column_width(col, width).map_err(excel)?;
        sheet
            .write_string_with_format(0, col, label, &formats.heading)
            .map_err(excel)?;
    }
    Ok(sheet)
}

fn write_cell(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    kind: Cell,
    value: &SqlValue,
    formats: &Formats,
) -> Result<()> {
    let cell = match (kind, value) {
        (_, SqlValue::Null) => return Ok(()),
        (Text, SqlValue::Text(text)) => {
            sheet.write_string_with_format(row, col, text, &formats.text)
        }
        (Id, SqlValue::Integer(n)) => {
            sheet.write_string_with_format(row, col, n.to_string(), &formats.text)
        }
        (Money, SqlValue::Integer(n)) => {
            sheet.write_number_with_format(row, col, *n as f64 / 100.0, &formats.money)
        }
        (Quantity, SqlValue::Integer(n)) => {
            sheet.write_number_with_format(row, col, *n as f64 / 10_000.0, formats.quantity(*n))
        }
        (Minimum, SqlValue::Text(text)) => {
            let Some(n) = catalog::wholesale_minimum(text) else {
                return Ok(());
            };
            sheet.write_number_with_format(row, col, n as f64 / 10_000.0, formats.quantity(n))
        }
        (Percent, SqlValue::Integer(n)) => {
            sheet.write_number_with_format(row, col, *n as f64 / 10000.0, &formats.percent)
        }
        (Date, SqlValue::Text(text)) => match ExcelDateTime::parse_from_str(text) {
            Ok(date) => sheet.write_datetime_with_format(
                row,
                col,
                &date,
                if text.len() == 10 {
                    &formats.date
                } else {
                    &formats.date_time
                },
            ),
            Err(_) => sheet.write_string(row, col, text),
        },
        (Unit, SqlValue::Integer(n)) => sheet.write_string(row, col, unit(*n)),
        (Payment, SqlValue::Integer(n)) => sheet.write_string(
            row,
            col,
            ["Efectivo", "Tarjeta", "Transferencia", "Crédito"]
                .get(*n as usize)
                .copied()
                .unwrap_or("Sin clasificar"),
        ),
        (Active, SqlValue::Integer(n)) => {
            sheet.write_string(row, col, if *n == 1 { "Activo" } else { "Archivado" })
        }
        (YesNo, SqlValue::Integer(n)) => {
            sheet.write_string(row, col, if *n == 1 { "Sí" } else { "No" })
        }
        _ => {
            return Err(AppError::rule(
                "EXPORT_VALUE",
                "Un dato no tiene el formato esperado para el archivo de Excel.",
            ));
        }
    };
    cell.map_err(excel)?;
    Ok(())
}

fn fill_sheet(
    book: &mut Workbook,
    db: &Connection,
    spec: &Sheet,
    date_range: Option<(&str, &str, &str)>,
    formats: &Formats,
) -> Result<()> {
    const BATCH_SIZE: usize = 2048;
    const ROW_LIMIT: u32 = 1_048_576;
    let projection = spec
        .columns
        .iter()
        .map(|(sql, _, _)| *sql)
        .collect::<Vec<_>>()
        .join(",");
    let date_filter = date_range.map_or_else(String::new, |(alias, _, _)| {
        format!("AND {alias}.CreatedAt>=?2 AND {alias}.CreatedAt<?3")
    });
    let query = format!(
        "SELECT r.Id,{projection} FROM {} WHERE r.Id>?1 {date_filter} ORDER BY r.Id LIMIT {BATCH_SIZE}",
        spec.from
    );
    let mut cursor = 0i64;
    let mut index = 1;
    let mut sheet = start_sheet(book, spec, index, formats)?;
    let mut row = 1;
    let last_col = (spec.columns.len() - 1) as u16;
    loop {
        let batch = match date_range {
            Some((_, start, end)) => db.rows(&query, params![cursor, start, end])?,
            None => db.rows(&query, [cursor])?,
        };
        let batch_len = batch.len();
        for record in batch {
            if row == ROW_LIMIT {
                sheet.autofilter(0, 0, row - 1, last_col).map_err(excel)?;
                index += 1;
                sheet = start_sheet(book, spec, index, formats)?;
                row = 1;
            }
            cursor = record.get(0)?;
            for (i, (_, _, kind)) in spec.columns.iter().enumerate() {
                write_cell(sheet, row, i as u16, *kind, &record.values[i + 1], formats)?;
            }
            row += 1;
        }
        if batch_len < BATCH_SIZE {
            break;
        }
    }
    if row > 1 {
        sheet.autofilter(0, 0, row - 1, last_col).map_err(excel)?;
    }
    Ok(())
}

impl Engine {
    pub fn export_business(&self, folder: &Path, scope: BusinessExport) -> Result<Value> {
        self.actor("settings")?;
        let (title, sheets) = scope.sheets();
        let day = if matches!(scope, BusinessExport::SalesToday) {
            let now = self.now_string();
            let today = NaiveDate::parse_from_str(&now[..10], "%Y-%m-%d").map_err(excel)?;
            let tomorrow = today
                .succ_opt()
                .ok_or_else(|| excel("Fecha fuera del rango de Excel"))?;
            Some((today.to_string(), tomorrow.to_string()))
        } else {
            None
        };
        let snapshot = self.db()?.unchecked_transaction()?;
        let formats = Formats::new();
        let mut book = Workbook::new();
        for (index, sheet) in sheets.iter().enumerate() {
            let date_range = day.as_ref().map(|(from, to)| {
                (
                    if index == 0 { "r" } else { "s" },
                    from.as_str(),
                    to.as_str(),
                )
            });
            fill_sheet(&mut book, &snapshot, sheet, date_range, &formats)?;
        }
        snapshot.commit()?;
        let filename =
            day.map_or_else(|| title.to_owned(), |(today, _)| format!("{title}-{today}"));
        save(&mut book, folder, &filename)
    }
}
