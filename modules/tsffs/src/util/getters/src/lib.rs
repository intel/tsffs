use darling::{ast::Data, util::Flag, FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Generics, Ident, Type};

#[derive(Debug, FromField)]
#[darling(attributes(getters))]
struct GettersField {
    ident: Option<Ident>,
    ty: Type,
    mutable: Flag,
    skip: Flag,
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(getters),
    supports(struct_named),
    forward_attrs(allow, doc, cfg)
)]
struct Getters {
    ident: Ident,
    generics: Generics,
    data: Data<(), GettersField>,
    mutable: Flag,
}

impl ToTokens for Getters {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let fields = self
            .data
            .as_ref()
            .take_struct()
            .expect("Expected named struct")
            .fields;

        let fields = fields
            .iter()
            .filter(|f| !f.skip.is_present())
            .map(|f| {
                let ident = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let mutable = f.mutable.is_present() || self.mutable.is_present();

                let immutable = quote! {
                    #[inline(always)]
                    /// Return a reference to the #ident field
                    pub fn #ident(&self) -> &#ty {
                        &self.#ident
                    }
                };

                if mutable {
                    let ident_mut = format_ident!("{}_mut", ident);
                    quote! {
                        #immutable

                        #[inline(always)]
                        /// Return a mutable reference to the #ident field
                        pub fn #ident_mut(&mut self) -> &mut #ty {
                            &mut self.#ident
                        }
                    }
                } else {
                    immutable
                }
            })
            .collect::<TokenStream2>();

        tokens.extend(quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                #fields
            }
        });
    }
}

#[proc_macro_derive(Getters, attributes(getters))]
#[allow(non_snake_case)]
/// Add immutable and (optionally) mutable accessors for every field of a struct
///
/// ```rust,ignore
/// use getters::Getters;
///
/// #[derive(Getters)]
/// pub struct Foo {
///     x: i32,
/// }
///
/// let f = Foo { x: 0 };
/// assert_eq!(f.x(), 0);
/// *f.eq_mut()  = 1;
/// assert_eq!(f.x(), 1);
/// ```
pub fn Getters(input: TokenStream) -> TokenStream {
    let getters = match Getters::from_derive_input(&parse_macro_input!(input as DeriveInput)) {
        Ok(g) => g,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let mut tokens = TokenStream2::new();

    getters.to_tokens(&mut tokens);

    tokens.into()
}
