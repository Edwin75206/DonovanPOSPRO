use crate::{
    Engine,
    error::{AppError, Result},
    models::*,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Mutex};
use tauri::Emitter;

pub struct AppState {
    pub engine: Mutex<Option<Result<Engine>>>,
    pub root: PathBuf,
    pub demo: bool,
}
#[derive(Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Request {
    Bootstrap,
    RetryStorage,
    Setup(Setup),
    Login {
        username: String,
        password: String,
    },
    Logout,
    Lock,
    Unlock {
        password: String,
    },
    DemoLogin {
        role: i64,
    },
    Products {
        search: String,
        category: String,
        status: String,
        page: i64,
    },
    SaveProduct(Product),
    AdjustStock {
        id: i64,
        delta: i64,
        reason: String,
    },
    StockHistory {
        id: i64,
    },
    Product {
        id: i64,
    },
    Inventory {
        search: String,
        category: String,
        status: String,
        page: i64,
        sort: String,
        descending: bool,
    },
    Price {
        items: Vec<SaleItem>,
        discount_parts: i64,
    },
    Checkout(Checkout),
    Sale {
        id: i64,
    },
    Sales {
        from: String,
        to: String,
        page: i64,
    },
    CashOverview,
    CashOpen {
        opening_cents: i64,
    },
    CashMovement {
        amount_cents: i64,
        reason: String,
    },
    CashClose {
        counted_cents: i64,
        notes: String,
    },
    Users,
    SaveUser {
        id: i64,
        username: String,
        name: String,
        role: i64,
        active: bool,
        password: String,
        admin_password: String,
    },
    Preferences {
        status: String,
        step: i64,
        theme: String,
        reduced_motion: bool,
    },
    Dashboard {
        from: String,
        to: String,
    },
    SaveAppearance {
        config: crate::appearance::Appearance,
        background_action: String,
        background_data: Option<String>,
    },
    ChooseAppearanceImage,
    Contacts {
        supplier: bool,
        search: String,
        archived: bool,
    },
    SaveContact {
        supplier: bool,
        contact: crate::contacts::Contact,
    },
    CustomerLedger {
        id: i64,
    },
    CreditPayment(crate::contacts::CreditPayment),
    Quotes {
        search: String,
    },
    Quote {
        id: i64,
    },
    SaveQuote(crate::quotes::QuoteInput),
    HeldSales,
    HeldSale {
        id: i64,
    },
    SaveHeld(crate::quotes::HeldInput),
    DiscardHeld {
        id: i64,
    },
    Purchases {
        search: String,
    },
    Purchase {
        id: i64,
    },
    ReceivePurchase(crate::purchasing::PurchaseInput),
    ReturnSale(crate::returns::ReturnInput),
    DataOverview,
    VerifyDatabase,
    CreateBackup {
        folder: String,
    },
    ChooseBackupFolder,
    ChooseRecoverySource,
    ConfirmRecovery {
        plan_id: String,
        username: String,
        password: String,
    },
    CancelRecovery,
    NewDatabase {
        password: String,
    },
    SaveBackupSettings {
        folder: String,
        enabled: bool,
        hours: i64,
        password: String,
    },
    Diagnostics,
    ChooseImportFile,
    ImportSheet {
        sheet: String,
    },
    ValidateImport(crate::importing::ImportOptions),
    CommitImport {
        plan_id: String,
    },
    ExportProducts {
        template: bool,
    },
}
fn dispatch(engine: &mut Engine, window: &tauri::WebviewWindow, request: Request) -> Result<Value> {
    match request {
        Request::Bootstrap => engine.bootstrap(),
        Request::RetryStorage => {
            engine.open_database()?;
            engine.bootstrap()
        }
        Request::Setup(input) => engine.setup(input),
        Request::Login { username, password } => engine.login(&username, &password),
        Request::Logout => engine.logout(),
        Request::Lock => engine.lock_screen(),
        Request::Unlock { password } => engine.unlock(&password),
        Request::DemoLogin { role } => engine.demo_login(role),
        Request::Products {
            search,
            category,
            status,
            page,
        } => engine.products(&search, &category, &status, page),
        Request::SaveProduct(input) => engine.save_product(input),
        Request::AdjustStock { id, delta, reason } => engine.adjust_stock(id, delta, &reason),
        Request::StockHistory { id } => engine.stock_history(id),
        Request::Product { id } => engine.get_product(id),
        Request::Inventory {
            search,
            category,
            status,
            page,
            sort,
            descending,
        } => engine.product_page(&search, &category, &status, page, &sort, descending),
        Request::Price {
            items,
            discount_parts,
        } => Ok(serde_json::to_value(engine.price(&items, discount_parts)?)?),
        Request::Checkout(request) => engine.checkout(&request),
        Request::Sale { id } => engine.sale(id),
        Request::Sales { from, to, page } => engine.sales(&from, &to, page),
        Request::CashOverview => engine.cash_overview(),
        Request::CashOpen { opening_cents } => engine.cash_open(opening_cents),
        Request::CashMovement {
            amount_cents,
            reason,
        } => engine.cash_movement(amount_cents, &reason),
        Request::CashClose {
            counted_cents,
            notes,
        } => engine.cash_close(counted_cents, &notes),
        Request::Users => engine.users(),
        Request::SaveUser {
            id,
            username,
            name,
            role,
            active,
            password,
            admin_password,
        } => engine.save_user(
            id,
            &username,
            &name,
            role,
            active,
            &password,
            &admin_password,
        ),
        Request::Preferences {
            status,
            step,
            theme,
            reduced_motion,
        } => engine.preferences(&status, step, &theme, reduced_motion),
        Request::Dashboard { from, to } => engine.dashboard(&from, &to),
        Request::SaveAppearance {
            config,
            background_action,
            background_data,
        } => engine.save_appearance(config, &background_action, background_data.as_deref()),
        Request::ChooseAppearanceImage => {
            engine.actor("sell")?;
            match rfd::FileDialog::new()
                .set_parent(window)
                .set_title("Elige un fondo para tu espacio")
                .add_filter("Imagen PNG o JPEG", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                Some(path) => engine.image_for_appearance(&path),
                None => Ok(Value::Null),
            }
        }
        Request::Contacts {
            supplier,
            search,
            archived,
        } => engine.contacts(supplier, &search, archived),
        Request::SaveContact { supplier, contact } => engine.save_contact(supplier, contact),
        Request::CustomerLedger { id } => engine.customer_ledger(id),
        Request::CreditPayment(input) => engine.credit_payment(input),
        Request::Quotes { search } => engine.quotes(&search),
        Request::Quote { id } => engine.quote(id),
        Request::SaveQuote(input) => engine.save_quote(input),
        Request::HeldSales => engine.held_sales(),
        Request::HeldSale { id } => engine.held_sale(id),
        Request::SaveHeld(input) => engine.save_held(input),
        Request::DiscardHeld { id } => engine.discard_held(id),
        Request::Purchases { search } => engine.purchases(&search),
        Request::Purchase { id } => engine.purchase(id),
        Request::ReceivePurchase(input) => engine.receive_purchase(input),
        Request::ReturnSale(input) => engine.return_sale(input),
        Request::DataOverview => engine.data_overview(),
        Request::VerifyDatabase => engine.verify_database(),
        Request::CreateBackup { folder } => engine.create_backup(&PathBuf::from(folder)),
        Request::ChooseImportFile => {
            engine.actor("catalog")?;
            match rfd::FileDialog::new()
                .set_parent(window)
                .set_title("Seleccionar productos de Excel")
                .add_filter("Libro de Excel", &["xlsx"])
                .pick_file()
            {
                Some(path) => engine.open_import(&path),
                None => Ok(Value::Null),
            }
        }
        Request::ImportSheet { sheet } => engine.import_sheet(&sheet),
        Request::ValidateImport(input) => engine.validate_import(input),
        Request::CommitImport { plan_id } => engine.commit_import(&plan_id),
        Request::ExportProducts { template } => {
            engine.actor("catalog")?;
            match rfd::FileDialog::new()
                .set_parent(window)
                .set_title(if template {
                    "Guardar plantilla de productos"
                } else {
                    "Exportar catálogo de productos"
                })
                .pick_folder()
            {
                Some(folder) => engine.export_products(&folder, template),
                None => Ok(Value::Null),
            }
        }
        Request::ChooseBackupFolder => {
            engine.actor("settings")?;
            Ok(rfd::FileDialog::new()
                .set_parent(window)
                .set_title("Carpeta para guardar respaldos")
                .pick_folder()
                .map(|p| json!(crate::storage::display_path(&p)))
                .unwrap_or(Value::Null))
        }
        Request::ChooseRecoverySource => {
            engine.recovery_access()?;
            match rfd::FileDialog::new()
                .set_parent(window)
                .set_title("Seleccionar respaldo o base de Donovan POS")
                .add_filter(
                    "Respaldo o base SQLite",
                    &["donovanbackup", "db", "sqlite", "sqlite3"],
                )
                .pick_file()
            {
                Some(path) => engine.prepare_recovery(&path),
                None => Ok(Value::Null),
            }
        }
        Request::ConfirmRecovery {
            plan_id,
            username,
            password,
        } => engine.confirm_recovery(&plan_id, &username, &password),
        Request::CancelRecovery => {
            engine.cancel_recovery()?;
            Ok(Value::Null)
        }
        Request::NewDatabase { password } => engine.create_new_database(&password),
        Request::SaveBackupSettings {
            folder,
            enabled,
            hours,
            password,
        } => engine.save_backup_settings(&PathBuf::from(folder), enabled, hours, &password),
        Request::Diagnostics => engine.diagnostics(),
    }
}
impl Request {
    fn operation(&self) -> &'static str {
        match self {
            Self::Login { .. } | Self::Unlock { .. } => "Inicio de sesión",
            Self::Setup(_) => "Configuración inicial",
            Self::SaveUser { .. } => "Guardar usuario",
            Self::Checkout(_) => "Cobrar venta",
            Self::ReturnSale(_) => "Devolver o cancelar venta",
            Self::ReceivePurchase(_) => "Recibir compra",
            Self::CreditPayment(_) => "Registrar abono",
            Self::SaveProduct(_) => "Guardar producto",
            Self::AdjustStock { .. } => "Ajustar inventario",
            Self::SaveQuote(_) => "Guardar cotización",
            Self::SaveHeld(_) => "Suspender venta",
            Self::ChooseRecoverySource => "Validar respaldo",
            Self::ConfirmRecovery { .. } => "Restaurar respaldo",
            Self::CreateBackup { .. } => "Crear respaldo",
            Self::NewDatabase { .. } => "Crear base",
            Self::VerifyDatabase => "Comprobar integridad",
            Self::SaveBackupSettings { .. } => "Configurar respaldos",
            Self::ChooseImportFile | Self::ImportSheet { .. } | Self::ValidateImport(_) => {
                "Revisar Excel"
            }
            Self::CommitImport { .. } => "Importar Excel",
            Self::ExportProducts { .. } => "Exportar catálogo",
            Self::SaveAppearance { .. } => "Guardar apariencia",
            Self::CashOpen { .. } | Self::CashMovement { .. } | Self::CashClose { .. } => {
                "Operación de caja"
            }
            _ => "Consulta local",
        }
    }
}
#[tauri::command]
pub async fn execute(
    state: tauri::State<'_, std::sync::Arc<AppState>>,
    window: tauri::WebviewWindow,
    request: Request,
) -> Result<Value> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
   let mut guard=state.engine.lock().map_err(|_|AppError::rule("UNEXPECTED","Ocurrió un problema. Cierra y vuelve a abrir Donovan POS Pro."))?;
   if guard.is_none()||(matches!(request,Request::RetryStorage)&&guard.as_ref().is_some_and(Result::is_err)){*guard=Some(Engine::open(&state.root).and_then(|mut engine|{if state.demo{engine.seed_demo()?;}Ok(engine)}));}
   let engine=match guard.as_mut().expect("engine initialized"){Ok(engine)=>engine,Err(error)=>{if matches!(request,Request::Bootstrap|Request::RetryStorage){return Ok(json!({"storageError":error,"version":"2.0.0"}));}return Err(error.clone());}};
   let operation=request.operation();let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||dispatch(engine,&window,request))).unwrap_or_else(|_|Err(AppError::rule("UNEXPECTED","Ocurrió un problema y no pudimos completar esta operación. Puedes volver a intentarlo.")));
   if let Err(error)=&result{engine.log_error("desktop",operation,error);if engine.startup_error.is_some(){let _=window.emit("storage-unavailable",());}}
   result
 }).await.map_err(|_|AppError::rule("UNEXPECTED","No pudimos completar la operación. Intenta nuevamente."))?
}
