mod protocol;
mod handler;
mod session;
mod allowed_origins;

use std::io;

fn main() {
    // Verify the calling extension is allowed
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let caller = &args[1];
        if !allowed_origins::is_allowed(caller) {
            eprintln!("Unauthorized caller: {}", caller);
            std::process::exit(1);
        }
    }
    
    // Set up panic hook to zeroize sensitive data
    std::panic::set_hook(Box::new(|info| {
        // Don't leak sensitive info in panic messages
        eprintln!("Internal error: {}", info);
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
