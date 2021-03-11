use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{parse, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident, Index};

#[proc_macro_derive(MemoryUsage)]
pub fn derive_memory_usage(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = parse(input).unwrap();

    match derive_input.data {
        Data::Struct(ref struct_data) => {
            derive_memory_usage_for_struct(&derive_input.ident, struct_data, &derive_input.generics)
        }

        Data::Enum(ref enum_data) => {
            derive_memory_usage_for_enum(&derive_input.ident, enum_data, &derive_input.generics)
        }

        Data::Union(_) => panic!("unions are not yet implemented"),
        /*
        // TODO: unions.
        // We have no way of knowing which union member is active, so we should
        // refuse to derive an impl except for unions where all members are
        // primitive types or arrays of them.
        Data::Union(ref union_data) => {
            derive_memory_usage_union(union_data)
        },
        */
    }
}

// TODO: use Iterator::fold_first once it's stable. https://github.com/rust-lang/rust/pull/79805
fn join_fold<I, F, B>(mut iter: I, function: F, empty: B) -> B
where
    I: Iterator<Item = B>,
    F: FnMut(B, I::Item) -> B,
{
    if let Some(first) = iter.next() {
        iter.fold(first, function)
    } else {
        empty
    }
}

fn derive_memory_usage_for_struct(
    struct_name: &Ident,
    data: &DataStruct,
    generics: &Generics,
) -> TokenStream {
    let lifetimes_and_generics = &generics.params;
    let where_clause = &generics.where_clause;

    let sum = join_fold(
        match &data.fields {
            Fields::Named(ref fields) => fields
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let span = ident.span();

                    quote_spanned!(
                        span => MemoryUsage::size_of_val(&self.#ident, visited) - std::mem::size_of_val(&self.#ident)
                    )
                })
                .collect(),

            Fields::Unit => vec![],

            Fields::Unnamed(ref fields) => (0..(fields.unnamed.iter().count()))
                .into_iter()
                .map(|field| {
                    let ident = Index::from(field);

                    quote! { MemoryUsage::size_of_val(&self.#ident, visited) - std::mem::size_of_val(&self.#ident) }
                })
                .collect(),
        }
        .iter()
        .cloned(), // TODO: shouldn't need cloned here
        |x, y| quote! { #x + #y },
        quote! { 0 },
    );

    (quote! {
        #[allow(dead_code)]
        impl < #lifetimes_and_generics > MemoryUsage for #struct_name < #lifetimes_and_generics >
        #where_clause
        {
            fn size_of_val(&self, visited: &mut MemoryUsageVisited) -> usize {
                std::mem::size_of_val(self) + #sum
            }
        }
    })
    .into()
}

fn derive_memory_usage_for_enum(
    struct_name: &Ident,
    data: &DataEnum,
    generics: &Generics,
) -> TokenStream {
    let lifetimes_and_generics = &generics.params;
    let where_clause = &generics.where_clause;

    let match_arms = join_fold(
        data.variants
            .iter()
            .map(|variant| {
                let ident = &variant.ident;
                let span = ident.span();

                let (pattern, sum) = match variant.fields {
                    Fields::Named(ref fields) => {
                        let identifiers = fields.named.iter().map(|field| {
                            let ident = field.ident.as_ref().unwrap();
                            let span = ident.span();

                            quote_spanned!(span => #ident)
                        });

                        let pattern =
                            join_fold(
                                identifiers.clone(),
                                |x, y| quote! { #x , #y },
                                quote! {}
                            );

                        let sum = join_fold(
                            identifiers.map(|ident| quote! { MemoryUsage::size_of_val(#ident, visited) - std::mem::size_of_val(#ident) }),
                            |x, y| quote! { #x + #y },
                            quote! { 0 },
                        );

                        (quote! { { #pattern } }, quote! { #sum })
                    }

                    Fields::Unit => (quote! {}, quote! { 0 }),

                    Fields::Unnamed(ref fields) => {
                        let identifiers =
                            (0..(fields.unnamed.iter().count()))
                            .into_iter()
                            .map(|field| {
                                let ident = Index::from(field);
                                let ident = format_ident!("value{}", ident);

                                quote! { #ident }
                            });

                        let pattern =
                            join_fold(
                                identifiers.clone(),
                                |x, y| quote! { #x , #y },
                                quote! {}
                            );

                        let sum = join_fold(
                            identifiers.map(|ident| quote! { MemoryUsage::size_of_val(#ident, visited) - std::mem::size_of_val(#ident) }),
                            |x, y| quote! { #x + #y },
                            quote! { 0 },
                        );
                        (quote! { ( #pattern ) }, quote! { #sum })
                    }
                };

                quote_spanned! { span=> Self::#ident#pattern => #sum }
            }
        ),
        |x, y| quote! { #x , #y },
        quote! {},
    );

    (quote! {
        #[allow(dead_code)]
        impl < #lifetimes_and_generics > MemoryUsage for #struct_name < #lifetimes_and_generics >
        #where_clause
        {
            fn size_of_val(&self, visited: &mut MemoryUsageVisited) -> usize {
                std::mem::size_of_val(self) + match self {
                    #match_arms
                }
            }
        }
    })
    .into()
}
