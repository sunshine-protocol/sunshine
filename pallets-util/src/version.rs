/// Simple version type
pub enum Version<T> {
    Major(T),
    MajorMinor(T, T),
    // MajorMinorPatch
    Semver(T, T, T),
}
