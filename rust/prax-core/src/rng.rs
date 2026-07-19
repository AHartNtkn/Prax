//! The drama die: a bit-exact MINSTD (Lehmer) stream. Advances unconditionally
//! on every draw (the frozen-die law: every draw spends one step, hit or miss),
//! then rolls on the advanced value. Seed domain guarded (0 and modulus
//! multiples are fixed points).
