use std::{future::Future, sync::Arc};

use gpui_kit::{Context, Task, Window};
use pitpls_app::App;
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct Services {
    pub app: Arc<App>,
    pub runtime: Arc<Runtime>,
}

impl Services {
    pub fn run<
        V: 'static,
        T: Send + 'static,
        F: Future<Output = Result<T, String>> + Send + 'static,
    >(
        &self,
        window: &Window,
        cx: &Context<V>,
        work: impl FnOnce(Arc<App>) -> F + Send + 'static,
        complete: impl FnOnce(&mut V, Result<T, String>, &mut Window, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let app = self.app.clone();
        let job = self.runtime.spawn(async move { work(app).await });
        cx.spawn_in(window, async move |view, cx| {
            let result = finish(job).await;
            // A closed page/window is an expected lifecycle outcome. The job can
            // finish without retaining that page or calling its former parent.
            let _ = view.update_in(cx, |view, window, cx| complete(view, result, window, cx));
        })
    }
}

pub async fn finish<T>(job: tokio::task::JoinHandle<Result<T, String>>) -> Result<T, String> {
    job.await
        .map_err(|error| format!("Background operation failed: {error}"))?
}
