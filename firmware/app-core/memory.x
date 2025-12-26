MEMORY
{
    /* NOTE 1 K = 1 KiBi = 1024 bytes */
    /* These values correspond to the NRF5340 */
    /* We have 1024K flash, the first 256K are available single-cycle */
    FLASH           : ORIGIN = 0x00000000, LENGTH = 256K
    /* Put the 'shared flash' in the last 256K of application core flash */
    SHARED_FLASH    : ORIGIN = 0x000C0000, LENGTH = 256K
    RAM             : ORIGIN = 0x20000000, LENGTH = 256K
    SHARED_RAM      : ORIGIN = 0x20040000, LENGTH = 256K
}

SECTIONS {

    .shared_ram (NOLOAD) : {
        *(.shared_ram)
        *(.shared_ram.*)
    } > SHARED_RAM
}
