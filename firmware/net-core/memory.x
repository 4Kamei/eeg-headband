ENTRY(Reset_Handler)

MEMORY
{
  FLASH (rx)  : ORIGIN =        0x01000000, LENGTH = 256K
  RAM   (rwx) : ORIGIN =        0x21000000, LENGTH = 64K
  SHARED_FLASH (rx) : ORIGIN =  0x000C0000, LENGTH = 256K
  SHARED_RAM (rwx) : ORIGIN =   0x20040000, LENGTH = 256K
}
SECTIONS {
    .shared_ram (NOLOAD) : {
        *(.shared_ram)
        *(.shared_ram.*)
    } > SHARED_RAM
}
