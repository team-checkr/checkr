#[wasm_bindgen]
pub struct WasmZ3 {
    ctx: String,
}

#[wasm_bindgen]
impl WasmZ3 {
    pub async fn new() -> Self {
        Self {
            ctx: init_context().await.as_string().unwrap(),
        }
    }
    pub async fn run(self) {
        use smtlib::AsyncSolver;
        let mut s = AsyncSolver::new(self).await.unwrap();
        let x = smtlib::Int::from_name("x");
        s.assert(x._eq(12)).await.unwrap();
        match s.check_sat_with_model().await.unwrap() {
            SatResultWithModel::Sat(m) => info!("{m}"),
            _ => warn!("No model produced!"),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl smtlib::backend::AsyncBackend for WasmZ3 {
    async fn exec(
        &mut self,
        cmd: &smtlib_lowlevel::ast::Command,
    ) -> Result<String, smtlib_lowlevel::Error> {
        let result = run(&self.ctx, &cmd.to_string()).await;
        result.as_string().ok_or_else(|| {
            smtlib_lowlevel::Error::IO(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "function did not return a string",
            ))
        })
    }
}

#[wasm_bindgen(module = "/z3-wrapper.js")]
extern "C" {
    async fn init_context() -> JsValue;
    async fn run(ctx: &str, cmd: &str) -> JsValue;
}
