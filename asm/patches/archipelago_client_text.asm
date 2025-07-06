.open "main.dol"
.org 0x80062e60 ; also called every frame but this properly displays text
bl print_archipelago_text

.close
