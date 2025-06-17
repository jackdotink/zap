mod ir;
mod luau;
mod types;

fn main() {
    let ty = match luau::run("return zap.u8()".as_bytes()) {
        Ok(ty) => ty,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    println!("Type: {:?}", ty);
}
