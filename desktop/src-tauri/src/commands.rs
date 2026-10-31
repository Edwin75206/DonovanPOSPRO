use crate::{Engine,error::{Result,AppError},models::*};
use serde::Deserialize;
use serde_json::{Value,json};
use std::{path::PathBuf,sync::Mutex};

pub struct AppState{pub engine:Mutex<Result<Engine>>,pub root:PathBuf}
#[derive(Deserialize)]
#[serde(tag="type",content="payload",rename_all="camelCase",rename_all_fields="camelCase")]
pub enum Request{
 Bootstrap,RetryStorage,Setup(Setup),Login{username:String,password:String},Logout,Lock,Unlock{password:String},DemoLogin{role:i64},
 Products{search:String,category:String,status:String,page:i64},SaveProduct(Product),AdjustStock{id:i64,delta:i64,reason:String},StockHistory{id:i64},
 Price{items:Vec<SaleItem>,discount_parts:i64},Checkout(Checkout),Sale{id:i64},Sales{from:String,to:String,page:i64},
 CashOverview,CashOpen{opening_cents:i64},CashMovement{amount_cents:i64,reason:String},CashClose{counted_cents:i64,notes:String},
 Users,SaveUser{id:i64,username:String,name:String,role:i64,active:bool,password:String,admin_password:String},
 Preferences{status:String,step:i64,theme:String,reduced_motion:bool},
}
fn dispatch(engine:&mut Engine,request:Request)->Result<Value>{
 match request{
  Request::Bootstrap=>engine.bootstrap(),Request::RetryStorage=>{engine.open_database()?;engine.bootstrap()},
  Request::Setup(input)=>engine.setup(input),Request::Login{username,password}=>engine.login(&username,&password),Request::Logout=>engine.logout(),Request::Lock=>engine.lock_screen(),Request::Unlock{password}=>engine.unlock(&password),Request::DemoLogin{role}=>engine.demo_login(role),
  Request::Products{search,category,status,page}=>engine.products(&search,&category,&status,page),Request::SaveProduct(input)=>engine.save_product(input),Request::AdjustStock{id,delta,reason}=>engine.adjust_stock(id,delta,&reason),Request::StockHistory{id}=>engine.stock_history(id),
  Request::Price{items,discount_parts}=>Ok(serde_json::to_value(engine.price(&items,discount_parts)?)?),Request::Checkout(request)=>engine.checkout(&request),Request::Sale{id}=>engine.sale(id),Request::Sales{from,to,page}=>engine.sales(&from,&to,page),
  Request::CashOverview=>engine.cash_overview(),Request::CashOpen{opening_cents}=>engine.cash_open(opening_cents),Request::CashMovement{amount_cents,reason}=>engine.cash_movement(amount_cents,&reason),Request::CashClose{counted_cents,notes}=>engine.cash_close(counted_cents,&notes),
  Request::Users=>engine.users(),Request::SaveUser{id,username,name,role,active,password,admin_password}=>engine.save_user(id,&username,&name,role,active,&password,&admin_password),
  Request::Preferences{status,step,theme,reduced_motion}=>engine.preferences(&status,step,&theme,reduced_motion),
 }
}
#[tauri::command]
pub async fn execute(state:tauri::State<'_,std::sync::Arc<AppState>>,request:Request)->Result<Value>{
 let state=state.inner().clone();
 tauri::async_runtime::spawn_blocking(move||{
   let mut guard=state.engine.lock().map_err(|_|AppError::rule("UNEXPECTED","Ocurrió un problema. Cierra y vuelve a abrir Donovan POS Pro."))?;
   if matches!(request,Request::RetryStorage)&&guard.is_err(){*guard=Engine::open(&state.root);}
   let engine=match guard.as_mut(){Ok(engine)=>engine,Err(error)=>{if matches!(request,Request::Bootstrap|Request::RetryStorage){return Ok(json!({"storageError":error,"version":"2.0.0"}));}return Err(error.clone());}};
   let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||dispatch(engine,request))).unwrap_or_else(|_|Err(AppError::rule("UNEXPECTED","Ocurrió un problema y no pudimos completar esta operación. Puedes volver a intentarlo.")));
   if let Err(error)=&result{engine.log_error("desktop","command",error);}
   result
 }).await.map_err(|_|AppError::rule("UNEXPECTED","No pudimos completar la operación. Intenta nuevamente."))?
}
