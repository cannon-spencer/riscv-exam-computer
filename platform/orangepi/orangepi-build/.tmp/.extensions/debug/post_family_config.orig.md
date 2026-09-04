*give the config a chance to override the family/arch defaults*
This hook is called after the family configuration (`sources/families/xxx.conf`) is sourced.
Since the family can override values from the user configuration and the board configuration,
it is often used to in turn override those.
