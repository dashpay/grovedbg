use tokio::sync::mpsc::channel;

// Desktop application version
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let Some(grovedbg_address) = std::env::var("GROVEDBG_ADDRESS")
        .ok()
        .and_then(|s| s.parse().ok())
    else {
        return eprintln!(
            "`GROVEDBG_ADDRESS` env variable must contain a URL or consider accessing GroveDBG web \
             interface directly"
        );
    };

    let rt = tokio::runtime::Runtime::new().expect("unable to create tokio runtime");

    egui_logger::builder().init().expect("unable to setup logger");

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    let (commands_sender, commands_receiver) = channel(5);
    let (updates_sender, updates_receiver) = channel(5);

    // Spawn a background task to process commands and push updates
    rt.spawn(grovedbg::start_grovedbg_protocol(
        grovedbg_address,
        commands_receiver,
        updates_sender,
    ));

    eframe::run_native(
        "GroveDBG",
        native_options,
        Box::new(|cc| {
            Ok(grovedbg::start_grovedbg_app(
                cc,
                commands_sender,
                updates_receiver,
            ))
        }),
    )
    .expect("Error starting GroveDBG");
}

// Web application version, served by a running GroveDB
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    egui_logger::builder().init().expect("unable to setup logger");

    let web_options = eframe::WebOptions::default();

    let (commands_sender, commands_receiver) = channel(5);
    let (updates_sender, updates_receiver) = channel(5);

    // Spawn a background task to process commands and push updates
    wasm_bindgen_futures::spawn_local(grovedbg::start_grovedbg_protocol(
        web_sys::window()
            .unwrap()
            .location()
            .origin()
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap(),
        commands_receiver,
        updates_sender,
    ));

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(grovedbg::start_grovedbg_app(
                        cc,
                        commands_sender,
                        updates_receiver,
                    ))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
