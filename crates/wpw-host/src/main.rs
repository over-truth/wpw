mod protocol;
mod handler;
mod session;
mod allowed_origins;

use std::io;

fn main() {
    // Verify the calling extension is allowed.
    // Chrome/Edge always pass `chrome-extension://<id>/` as argv[1]; if it's missing
    // we are not running under a browser and refuse to proceed.
    let args: Vec<String> = std::env::args().collect();
    let caller = match args.get(1) {
        Some(c) => c,
        None => {
            eprintln!("Missing caller origin argument");
            std::process::exit(1);
        }
    };
    if !allowed_origins::is_allowed(caller) {
        eprintln!("Unauthorized caller");
        std::process::exit(1);
    }

    // Set up panic hook — keep output fixed to avoid leaking sensitive context.
    std::panic::set_hook(Box::new(|_info| {
        eprintln!("Internal error");
    }));
    
    loop {
        match protocol::read_message() {
            Ok(request) => {
                let response = handler::handle_request(&request);
                if let Err(e) = protocol::write_message(&response) {
                    eprintln!("Failed to write response: {}", e);
                    break;
                }
            }
            Err(e) => {
                // EOF means the browser closed the connection
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                }
                eprintln!("Failed to read message: {}", e);
                break;
            }
        }
    }
}
