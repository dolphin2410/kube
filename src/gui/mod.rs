use crate::env;
use crate::zip::unzip;
use tokio::fs::{self, File};
use tokio::runtime::Builder;
use tokio::io::{AsyncWriteExt, BufWriter};
use web_view::{Content, WebView};
use std::path::Path;
use nfd::Response;

/// The bytes of the zip file to be installed
const ZIP_BYTES: &[u8] = include_bytes!(concat!("../../", include_str!("../../target.txt")));

/// The application name to install
const APP_NAME: &str = include_str!("../../app.txt");

/// The root HTML
pub fn root() -> &'static str {
    include_str!("../../resources/root.html")
}

/// The final page HTML
pub fn next_page() -> &'static str {
    include_str!("../../resources/next_page.html")
}

pub async fn handle(pathname_str: &str, webview: &mut WebView<'_, ()>) {

    // The name of the target zip file that the bytes will be stored
    let zip_filename = &format!("{}/{}.zip", pathname_str, APP_NAME);

    // The zip file where the bytes are stored
    let mut zipfile = BufWriter::new(File::create(zip_filename).await.unwrap());

    // Write bytes to the zip file
    zipfile.write_all(ZIP_BYTES).await.unwrap();
    
    // Finish Write
    zipfile.flush().await.unwrap();

    // 20% Done
    webview.eval(&format!("update_size(20)")).unwrap();

    // The name of the folder where the archive will be extracted
    let folder_name = &format!("{}/{}/", pathname_str, APP_NAME);

    // Create the folder where the zip contents will be extracted
    fs::create_dir(folder_name).await.unwrap();

    unzip(zip_filename, folder_name);

    // 80% Done
    webview.eval(&format!("update_size(80)")).unwrap();

    // Save Environment variable to the 'bin' folder
    env::add_path(&format!("{}{}", folder_name, "bin"));

    // Delete the zipfile
    fs::remove_file(Path::new(zip_filename)).await.unwrap();

    // 100% Complete!
    webview.eval(&format!("update_size(100)")).unwrap();

    // Enable Button
    webview.eval(&format!("enable_btn()")).unwrap();

}

/// Render the WebView
pub fn render(html: &str) {
    web_view::builder()
        .title("kube")
        .content(Content::Html(html))
        .size(800, 500)
        .resizable(false)
        .user_data(())
        .invoke_handler(|webview, arg| {
            match arg {
                // Open the file exploring request
                "open_file" => {
                    let result = nfd::open_pick_folder(None).unwrap();
                    match result {
                        Response::Okay(path) => {
                            webview
                                .eval(&format!(r#"update_folder("{}")"#, path.replace("\\", "/")))
                                .unwrap();
                        }
                        _ => {}
                    }
                }

                // Exit the view
                "exit" => {
                    webview.exit();
                }
                _ => {
                    if arg.starts_with("next_page:") {
                        // Read the pathname
                        let pathname = arg.replace("next_page:", "");

                        // Pathname to &str
                        let pathname_str = pathname.as_str();

                        // The path
                        let path = Path::new(pathname_str);

                        // Check whether the code should stop
                        let mut success = true;

                        // Parent
                        if let Some(parent) = path.parent() {
                            // If the parent folder doesn't exist
                            if !parent.exists() {
                                // Send a warning that says the parent folder doesn't exist
                                webview.eval("no_file()").unwrap();

                                // Stop here
                                success = false;
                            }
                        } else {
                            // Stop
                            success = false;
                        }

                        // Run if the code isn't broken in the middle
                        if success {
                            // Whether the path exists
                            let exists = path.exists();

                            // Execute if the folder doesn't exist or if the folder is empty
                            if !exists || (exists && path.read_dir().unwrap().next().is_none()) {
                                // If the folder doesn't exist, create a new one
                                if !exists {
                                    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
                                    runtime.block_on(async {
                                        fs::create_dir(path).await.unwrap();
                                    });
                                }

                                // The next page HTML
                                webview
                                    .eval(&format!(
                                        "move_page('{}')",
                                        next_page().replace("\r", "").replace("\n", "\\n")
                                    ))
                                    .unwrap();

                                // TODO Fix block_on
                                Builder::new_multi_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap()
                                    .block_on(handle(pathname_str, webview));

                            // Execute if the folder exists and isn't empty
                            } else {
                                // Show warning that the file already exists
                                webview.eval("yes_file()").unwrap();
                            }
                        }
                    }
                }
            };

            Ok(())
        })
        .build()
        .unwrap()
        .run()
        .unwrap();
}

/// Start the GUI
pub fn start() {
    render(root());
}
