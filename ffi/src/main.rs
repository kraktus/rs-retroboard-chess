use retroboard::RetroBoard;

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type RetroBoardFFI;
    }
}

struct RetroBoardFFI(RetroBoard);

fn main() {}
