use axum::{routing::post, Json, Router};
use opener::OpenError;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub fn router() -> Router {
    Router::new()
        .route("/open", post(open_handler))
        .route("/print", post(print_handler))
}

#[derive(Deserialize)]
struct OpenFolder {
    mode: String,
    path: Vec<String>,
}

async fn open_handler(Json(OpenFolder { mode, path }): Json<OpenFolder>) -> Json<String> {
    let path_buf: PathBuf = path.iter().collect();
    let path: &Path = path_buf.as_path();

    match mode.as_str() {
        "open" => match opener::open(path) {
            Ok(_) => Json("OK".into()),
            Err(e) => {
                println!("Failed to open folder: {}", e);

                match e {
                    OpenError::Io(e) => {
                        println!("IO error: {}", e);
                        Json("IO Error".into())
                    }
                    _ => Json(e.to_string()),
                }
            }
        },
        "reveal" => match opener::reveal(path) {
            Ok(_) => Json("OK".into()),
            Err(e) => Json(e.to_string()),
        },
        _ => Json("Unknown mode".into()),
    }
}

#[derive(Deserialize)]
struct PrintFolder {
    path: Vec<String>,
}

#[cfg(target_os = "windows")]
use winprint::{
    printer::{FilePrinter, PdfiumPrinter, PrinterDevice},
    ticket::{
        FeatureOptionPackWithPredefined, PredefinedMediaName, PredefinedPageOutputColor,
        PrintCapabilities, PrintTicketBuilder,
    },
};

#[allow(unused)]
async fn print_handler(Json(PrintFolder { path }): Json<PrintFolder>) -> Json<String> {
    let path_buf: PathBuf = path.iter().collect();
    let path: &Path = path_buf.as_path();
    let path = path.to_str().unwrap();

    #[cfg(target_os = "windows")]
    print_file(&path);

    Json("OK".into())
}

#[cfg(target_os = "windows")]
fn print_file(path: &str) {
    let printers = PrinterDevice::all().expect("Failed to get printers");
    let my_device = printers
        .into_iter()
        .find(|x| x.name().contains("HP PageWide Color MFP E77650"))
        .expect("My Printer not found");

    let capabilities = PrintCapabilities::fetch(&my_device).unwrap();
    // println!("Capabilities: {:#?}", capabilities); // capabilities

    let a4_media = capabilities
        .page_media_sizes()
        .find(|x| x.as_predefined_name() == Some(PredefinedMediaName::ISOA4))
        .unwrap();

    let monochrome_media = capabilities
        .page_output_colors()
        .find(|x| x.as_predefined_name() == Some(PredefinedPageOutputColor::Grayscale))
        .unwrap();

    let mut builder = PrintTicketBuilder::new(&my_device).unwrap();
    builder.merge(a4_media).unwrap();
    builder.merge(monochrome_media).unwrap();
    let ticket = builder.build().unwrap();
    println!(
        "{}",
        String::from_utf8(ticket.get_xml().to_vec()).unwrap_or("NOTHING TO SEE".into())
    );

    let pdf = PdfiumPrinter::new(my_device);
    let path = std::path::Path::new(path);
    pdf.print(path, ticket).unwrap();
}
