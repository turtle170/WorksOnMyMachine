//! Windows builds get version metadata (ProductName, ProductVersion), which
//! SignPath requires on every signed binary. Other platforms get nothing.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "womm");
        res.set("FileDescription", "womm: enterprise-grade declarative executable synthesis");
        res.set("CompanyName", "turtle170");
        res.set("LegalCopyright", "MIT License. Works on my machine.");
        res.compile().expect("could not embed the Windows version resource");
    }
}
