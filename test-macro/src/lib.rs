use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, parse_quote, Block, ItemFn};

#[proc_macro_attribute]
pub fn enable_logging(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);

    let name = function.sig.ident.to_string();
    let stmts = function.block.stmts;
    let block: Block = parse_quote! {{
        if ::std::env::args().any(|e| e == "--nocapture") {
            use tracing_subscriber::fmt::format::FmtSpan;

            tracing_subscriber::fmt()
                .pretty()
                .compact()
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .with_env_filter(
                    ::tracing_subscriber::EnvFilter::builder()
                        .with_default_directive(::tracing::level_filters::LevelFilter::INFO.into())
                        .from_env_lossy(),
                )
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .init();
            let span = ::tracing::span!(::tracing::Level::INFO, #name).entered();

            #(#stmts)*
        } else {
            #(#stmts)*
        };
    }};
    function.block = Box::new(block);

    function.into_token_stream().into()
}
