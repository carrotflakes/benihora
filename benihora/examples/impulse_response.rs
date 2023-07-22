// cargo run --release --example impulse_response 2> /dev/null > impulse_reseponse.raw
// sox -t raw -e signed-integer -b 16 -r 48000 -c 1 impulse_reseponse.raw impulse_reseponse.wav

use std::io::Write;

use benihora::{tract_impulse_response, Benihora};

fn main() {
    let benihora = Benihora::new(3.0, 48000.0, 1.0, 0, false);
    let buf = tract_impulse_response(48000, &benihora.tract);

    let mut stdout = std::io::stdout();
    for x in buf {
        let x = (x * std::i16::MAX as f64) as i16;
        stdout.write(&x.to_ne_bytes()).unwrap();
    }
}
