use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{self, Parser},
    parse_macro_input, Error, Fields, ItemStruct,
};

#[proc_macro_attribute]
pub fn listing(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemStruct);
    let _ = parse_macro_input!(args as parse::Nothing);

    let struct_name = &input.ident;
    let exclude_fields = [
        "current_page",
        "max_pages",
        "msg",
        "embed",
        "entries_per_page",
    ];

    // Adding required fields
    // 1. max_pages
    // 2. current_page
    if let syn::Fields::Named(ref mut fields) = input.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub current_page: usize })
                .unwrap(),
        );

        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub max_pages: usize })
                .unwrap(),
        );

        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub entries_per_page: usize })
                .unwrap(),
        );

        fields.named.push(
            syn::Field::parse_named
                .parse2(
                    quote! { pub msg: fumo_twilight::message::MessageBuilder },
                )
                .unwrap(),
        );

        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! {
                    pub embed:
                        Option<twilight_model::channel::message::embed::Embed>
                })
                .unwrap(),
        );
    }

    let fields = if let Fields::Named(fields) = &input.fields {
        fields.named.iter().collect::<Vec<_>>()
    } else {
        // panic!("Only named fields are supported"); // TODO REMOVE PANIC
        return Error::new(
            Span::mixed_site(),
            "Only named fields are supported",
        )
        .into_compile_error()
        .into();
    };

    let field_names: Vec<_> = fields
        .iter()
        .filter_map(|f| {
            if let Some(ident) = &f.ident {
                if exclude_fields.iter().any(|x| x == &ident.to_string()) {
                    // TODO: remove this tupid Allocations
                    return None;
                } else {
                    return Some(&f.ident);
                }
            }

            None
        })
        .collect();

    let field_types: Vec<_> = fields
        .iter()
        .filter_map(|f| {
            if let Some(ident) = &f.ident {
                if exclude_fields.iter().any(|x| x == &ident.to_string()) {
                    // TODO: remove this tupid Allocations
                    return None;
                } else {
                    return Some(&f.ty);
                }
            }

            None
        })
        .collect();

    // Page navigation methods
    let methods = quote! {
        impl #struct_name {
            pub fn new(#(#field_names: #field_types),*) -> Self {
                Self {
                    current_page: 1,
                    max_pages: 20,
                    entries_per_page: 10,
                    msg: Default::default(),
                    embed: None,
                    #(#field_names),*
                }
            }


            pub fn max_pages(mut self, max_pages: usize) -> Self {
                self.max_pages = max_pages;

                self
            }

            pub fn entries_per_page(mut self, entries_per_page: usize) -> Self {
                self.entries_per_page = entries_per_page;

                self
            }

            /// Calculates and sets [`max_pages`] from
            /// requested [`entries_per_page`]
            pub fn calculate_pages(
                mut self,
                len: usize, entries_per_page: usize
            ) -> Self {
                self.entries_per_page = entries_per_page;
                self.max_pages =
                    (len as f32 / entries_per_page as f32).ceil() as usize;

                self
            }

            fn next_page(&mut self) {
                self.current_page += 1;
                if self.current_page > self.max_pages {
                    self.current_page = self.max_pages;
                }
            }

            fn previous_page(&mut self) {
                self.current_page -= 1;
                if self.current_page < 1 {
                    self.current_page = 1;
                }
            }
        }
    };

    quote! {
        #input
        #methods
    }
    .into()
}
