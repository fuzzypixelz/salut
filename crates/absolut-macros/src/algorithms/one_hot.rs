extern crate proc_macro;

use std::{
    collections::{HashMap, HashSet},
    iter,
};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};

use quote::quote;

use crate::{algorithms::Nibbles, driver::Builder};

use super::{Bytes, Class};

const LANES: usize = 16;

pub struct OneHotBuilder {
    table: HashMap<Bytes, Class>,
}

impl Builder for OneHotBuilder {
    fn new(_args: TokenStream) -> Self {
        Self {
            table: HashMap::with_capacity(256),
        }
    }

    fn insert(&mut self, bytes: Bytes, class: Class) {
        let _ = self.table.insert(bytes, class);
    }

    fn build(
        self,
        wildcard: Class,
        ident: syn::Ident,
        span: proc_macro2::Span,
    ) -> syn::Result<proc_macro2::TokenStream> {
        assert!(self.table.len() /* wildcard */ <= 8);

        fn is_product(bytes: &[u8]) -> bool {
            let bytes = bytes
                .iter()
                .map(|b| Nibbles::from_byte(*b))
                .collect::<HashSet<_>>();
            let (mut los, mut his) = (HashSet::new(), HashSet::new());
            let mut missing = HashSet::new();

            for Nibbles { lo, hi } in bytes.iter().copied() {
                if !los.contains(&lo) {
                    for lo in los.iter().copied() {
                        let n = Nibbles { lo, hi };
                        if !bytes.contains(&n) {
                            missing.insert(n.into_byte());
                        }
                    }

                    let _ = los.insert(lo);
                }

                if !his.contains(&hi) {
                    for hi in his.iter().copied() {
                        let n = Nibbles { lo, hi };
                        if !bytes.contains(&n) {
                            missing.insert(n.into_byte());
                        }
                    }

                    let _ = his.insert(hi);
                }
            }

            missing.is_empty()
        }

        let mut table_lo = [0_u8; LANES];
        let mut table_hi = [0_u8; LANES];

        for (i, (bytes, _)) in self.table.iter().enumerate() {
            let bytes = bytes.to_vec();

            if !is_product(&bytes) {
                // TODO(fuzzypixelz): Set message here
                return Err(syn::Error::new(span, "..."));
            }

            for byte in bytes {
                let Nibbles { lo, hi } = Nibbles::from_byte(byte);

                table_lo[lo as usize] |= 1 << i;
                table_hi[hi as usize] |= 1 << i;
            }
        }

        let variants = self
            .table
            .values()
            .enumerate()
            .map(|(i, class)| (class.name.as_str(), 1_u8 << i))
            .chain(iter::once((wildcard.name.as_str(), 0_u8)))
            .map(|(name, value)| {
                let name = Ident::new(name, Span::call_site());
                quote!(#name = #value)
            });

        let syntax = quote! {
            enum #ident {
                #(#variants,
                )*
            }

            impl ::absolut::OneHot for #ident {
                const LO: [u8; #LANES] = [#(#table_lo, )*];
                const HI: [u8; #LANES] = [#(#table_hi, )*];
            }
        };

        Ok(syntax)
    }
}
