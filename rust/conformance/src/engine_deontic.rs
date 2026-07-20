//! The EngineSpec pin that needs both the prax-core engine and prax-vocab's
//! `obliged_close` in scope (prax-core cannot depend on prax-vocab, so it homes
//! here): a world that DECLARES its □-closure derives the sub-obligation through
//! the ordinary engine, no engine gate and no census.

#[cfg(test)]
mod tests {
    use prax_core::engine::State;
    use prax_core::query::Condition;
    use prax_core::types::{Axiom, insert};
    use prax_vocab::deontic::obliged_close;

    // H: EngineSpec.hs "obligedClose lets a world close an obliged context (□a ⊢ □b)"
    #[test]
    fn obliged_close_lets_a_world_close_an_obliged_context() {
        // The world declares its closure with obliged_close; the engine closes
        // over the expanded list, so an obliged context derives the
        // sub-obligation (DEON property 1).
        let mut st = State::new();
        let lift_ax = Axiom::new(vec![Condition::Match("a.X".into())], ["b.X"]);
        st.set_axioms(obliged_close(&[lift_ax])).unwrap();
        st.perform_outcome(&insert("obliged.w.a.foo")).unwrap();
        assert!(
            st.view_has("obliged.w.b.foo"),
            "sub-obligation derived (□a ⊢ □b)"
        );
    }
}
