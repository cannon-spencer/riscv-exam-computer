*For final user override, using a function, after all aggregations are done*
Called after aggregating all package lists, before the end of `compilation.sh`.
Packages will still be installed after this is called, so it is the last chance
to confirm or change any packages.
