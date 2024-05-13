bftools.exe [-h | --help] [miz] 

$ ./bftools.exe miz --help
Usage: bftools.exe miz [OPTIONS] --output <OUTPUT> --base <BASE> --weapon <WEAPON> --options <OPTIONS>

Options:
  	--output <OUTPUT>                                  	the final miz file to output
  	--base <BASE>                                      	the base mission file
  	--weapon <WEAPON>                                  	the weapon template
  	--options <OPTIONS>                                	the options template
  	--warehouse <WAREHOUSE>                            	the warehouse template
  	--blue-production-template <BLUE_PRODUCTION_TEMPLATE>  [default: BINVENTORY]
  	--red-production-template <RED_PRODUCTION_TEMPLATE>	[default: RINVENTORY]
  -h, --help                                             	Print help
  
  
 EXAMPLE:
 $ cd ${HOME}/Saved Games/DCS.openbeta/Missions/SouthAtlantic

$ bftools.exe miz --output SouthAtlantic_final.miz --base SouthAtlantic_base.miz --weapon SouthAtlantic_weapons.miz --options SouthAtlantic_options.miz --warehouse SouthAtlantic_warehouse.miz
