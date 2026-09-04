*allow extensions to prepare their own config, after user config is done*
Implementors should preserve variable values pre-set, but can default values an/or validate them.
This runs *after* user_config. Don't change anything not coming from other variables or meant to be configured by the user.
