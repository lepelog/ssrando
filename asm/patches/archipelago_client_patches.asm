.open "main.dol"
.org 0x80062e60 ; also called every frame but this properly displays text
bl custom_main_additions

.close
