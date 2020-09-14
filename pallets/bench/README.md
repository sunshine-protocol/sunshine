# Sunshine Common Benchmarking

## Map with OrderedSet Value vs DoubleMap using Keys as HashSet

The expectation is that for small set sizes, it will be better to use a Map with OrderedSet. For larger sets, set membership lookups may be substantially cheaper for the DoubleMap using Keys as HashSet.