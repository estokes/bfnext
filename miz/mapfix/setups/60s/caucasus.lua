_aircraft = {
    ["Ka-50_3"]= { --This may need to be renamed to BS3
        ["default"] = {
            ["livery_id"] = "Russia Standard Army",
            ["AddPropAircraft"] = 
            {
                ["IMU alignment type"] = 3,
                ["modification"] = "Ka-50_3",
                ["Helmet-mounted device"] = 0,
                ["Realistic INS"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{A6FD14D3-6D30-4C85-88A7-8D17BEE120E2}",
                    }, -- end of [1]
                    [2] = 
                    {
                        ["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
                    }, -- end of [2]
                    [3] = 
                    {
                        ["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
                    }, -- end of [3]
                    [4] = 
                    {
                        ["CLSID"] = "{A6FD14D3-6D30-4C85-88A7-8D17BEE120E2}",
                    }, -- end of [4]
					[5] = 
                    {
                        ["CLSID"] = "{9S846_2xIGLA}",
                    }, -- end of [5]
                    [6] = 
                    {
                        ["CLSID"] = "{9S846_2xIGLA}",
                    }, -- end of [6]
                }, -- end of ["pylons"]
                ["fuel"] = "870",
                ["flare"] = 128,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["Radio"] = 
            {
                [1] = 
                {
                    ["modulations"] = 
                    {
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [1] = 30,
                        [2] = 35,
                        [3] = 27,
                        [4] = 28,
                        [5] = 30,
                        [6] = 32,
                        [7] = 40,
                        [8] = 50,
                        [9] = 55.5,
                        [10] = 59.9,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] = 
                {
                    ["modulations"] = 
                    {
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [1] = 0.625,
                        [2] = 0.303,
                        [3] = 0.289,
                        [4] = 0.591,
                        [5] = 0.408,
                        [6] = 0.803,
                        [7] = 0.443,
                        [8] = 0.215,
                        [9] = 0.525,
                        [10] = 1.065,
                        [11] = 0.718,
                        [12] = 0.35,
                        [13] = 0.583,
                        [14] = 0.283,
                        [15] = 0.995,
                        [16] = 1.21,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["Mi-24P"]= {
        ["default"] = {
            ["livery_id"] = "georgian air force",
            ["AddPropAircraft"] =
            {
                ["LeftEngineResource"] = 90,
                ["RightEngineResource"] = 90,
                ["SimplifiedAI"] = false,
                ["ExhaustScreen"] = true,
                ["NetCrewControlPriority"] = 0,
                ["GunnersAISkill"] = 90,
                ["HideAngleBoxes"] = false,
                ["NS430allow"] = false,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
			{
				["pylons"] = 
				{
					[1] = 
					{
						["CLSID"] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
					}, -- end of [1]
					[2] = 
					{
						["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
					}, -- end of [2]
					[3] = 
					{
						["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
					}, -- end of [3]
					[4] = 
					{
						["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
					}, -- end of [4]
					[5] = 
					{
						["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
					}, -- end of [5]
					[6] = 
					{
						["CLSID"] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
					}, -- end of [6]
				}, -- end of ["pylons"]
				["fuel"] = 1021,
				["flare"] = 192,
				["ammo_type"] = 1,
				["chaff"] = 0,
				["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{2x9M120F_Ataka_V}",
						[2] = "{2x9M120_Ataka_V}",
						[3] = "{2x9M220_Ataka_V}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{2x9M120F_Ataka_V}",
						[2] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
						[3] = "{2x9M120_Ataka_V}",
						[4] = "{2x9M220_Ataka_V}",
						[5] = "{APU-60-1_R_60M}",
						[6] = "{B0DBC591-0F52-4F7D-AD7B-51E67725FB81}",
					}, -- end of [2]
					[5] = 
					{
						[1] = "{2x9M120F_Ataka_V}",
						[2] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
						[3] = "{2x9M120_Ataka_V}",
						[4] = "{2x9M220_Ataka_V}",
						[5] = "{APU-60-1_R_60M}",
						[6] = "{275A2855-4A79-4B2D-B082-91EA2ADF4691}",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{2x9M120F_Ataka_V}",
						[2] = "{2x9M120_Ataka_V}",
						[3] = "{2x9M220_Ataka_V}",
					}, -- end of [6]
				}, -- end of ["restricted"]
            }, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 124,
                        [3] = 136,
                        [4] = 127,
                        [5] = 125,
                        [6] = 121,
                        [7] = 141,
                        [8] = 128,
                        [9] = 126,
                        [10] = 133,
                        [11] = 130,
                        [12] = 129,
                        [13] = 123,
                        [14] = 131,
                        [15] = 134,
                        [16] = 132,
                        [17] = 138,
                        [18] = 122,
                        [19] = 124,
                        [20] = 137,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 35,
                        [2] = 25.7,
                        [3] = 27,
                        [4] = 28,
                        [5] = 30,
                        [6] = 32,
                        [7] = 40,
                        [8] = 50,
                        [9] = 55.5,
                        [10] = 59.9,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["Mi-8MT"]= {
        ["default"] = {
            -- ["livery_id"] = "Russia_VVS_Standard",
            ["AddPropAircraft"] =
            {
                ["LeftEngineResource"] = 90,
                ["RightEngineResource"] = 90,
                ["NetCrewControlPriority"] = 1,
                ["ExhaustScreen"] = true,
                ["CargoHalfdoor"] = true,
                ["GunnersAISkill"] = 90,
                ["AdditionalArmor"] = true,
                ["NS430allow"] = false,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{05544F1A-C39C-466b-BC37-5BD1D52E57BB}",
                    }, -- end of [2]
                    [5] =
                    {
                        ["CLSID"] = "{05544F1A-C39C-466b-BC37-5BD1D52E57BB}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{6A4B9E69-64FE-439a-9163-3A87FB6A4D81}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "KORD_12_7",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "PKT_7_62",
                    }, -- end of [8]
                }, -- end of ["pylons"]
                ["fuel"] = "1929",
                ["flare"] = 128,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 124,
                        [3] = 136,
                        [4] = 127,
                        [5] = 125,
                        [6] = 121,
                        [7] = 141,
                        [8] = 128,
                        [9] = 126,
                        [10] = 133,
                        [11] = 130,
                        [12] = 129,
                        [13] = 123,
                        [14] = 131,
                        [15] = 134,
                        [16] = 132,
                        [17] = 138,
                        [18] = 122,
                        [19] = 124,
                        [20] = 137,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 35,
                        [2] = 25.7,
                        [3] = 27,
                        [4] = 28,
                        [5] = 30,
                        [6] = 32,
                        [7] = 40,
                        [8] = 50,
                        [9] = 55.5,
                        [10] = 59.9,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["UH-1H"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["SoloFlight"] = false,
                ["ExhaustScreen"] = true,
                ["GunnersAISkill"] = 90,
                ["NetCrewControlPriority"] = 0,
                ["EngineResource"] = 90,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "M134_L",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "XM158_MK5",
                    }, -- end of [2]
                    [6] =
                    {
                        ["CLSID"] = "M134_R",
                    }, -- end of [6]
                    [5] =
                    {
                        ["CLSID"] = "XM158_MK5",
                    }, -- end of [5]
                }, -- end of ["pylons"]
                ["fuel"] = "631",
                ["flare"] = 60,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [3] = 265,
                        [4] = 256,
                        [5] = 254,
                        [6] = 250,
                        [7] = 270,
                        [8] = 257,
                        [9] = 255,
                        [10] = 262,
                        [11] = 259,
                        [12] = 268,
                        [13] = 269,
                        [14] = 260,
                        [15] = 263,
                        [16] = 261,
                        [17] = 267,
                        [18] = 251,
                        [19] = 253,
                        [20] = 266,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
            -- ["livery_id"] = "US ARMY 1972",
        }
    },
    ["SA342M"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["NS430allow"] = false,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{HOT3D}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{HOT3G}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{HOT3D}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{HOT3G}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{FAS}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{IR_Deflector}",
                    }, -- end of [6]
                }, -- end of ["pylons"]
                ["fuel"] = 416.33,
                ["flare"] = 32,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            -- ["livery_id"] = "Combat",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 35,
                        [2] = 31,
                        [3] = 32,
                        [4] = 33,
                        [5] = 40,
                        [6] = 41,
                        [7] = 42,
                        [8] = 50,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["A-10A"]= {
        ["default"] = {
			["livery_id"] = "104th fs maryland ang, baltimore (md)",
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{3C0745ED-8B0B-42eb-B907-5BD5C1717447}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{69926055-0DA8-4530-9F2F-C86B157EA9F6}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{69926055-0DA8-4530-9F2F-C86B157EA9F6}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{60CC734F-0AFA-4E2E-82B8-93B941AB11CF}",
                    }, -- end of [4]
                    [11] =
                    {
                        ["CLSID"] = "{3C0745ED-8B0B-42eb-B907-5BD5C1717447}",
                    }, -- end of [11]
                    [10] =
                    {
                        ["CLSID"] = "{69926055-0DA8-4530-9F2F-C86B157EA9F6}",
                    }, -- end of [10]
                    [9] =
                    {
                        ["CLSID"] = "{69926055-0DA8-4530-9F2F-C86B157EA9F6}",
                    }, -- end of [9]
                    [8] =
                    {
                        ["CLSID"] = "{60CC734F-0AFA-4E2E-82B8-93B941AB11CF}",
                    }, -- end of [8]
                }, -- end of ["pylons"]
                ["fuel"] = 3017,
                ["flare"] = 120,
                ["ammo_type"] = 1,
                ["chaff"] = 240,
                ["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{BDU-50LD}",
						[2] = "ALQ_184",
						[3] = "LAU-105_AIS_ASQ_T50_L",
						[4] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[5] = "LAU-105_2*AIM-9P5",
						[6] = "{BDU-50HD}",
						[7] = "{6D21ECEA-F85B-4E8D-9D51-31DC9B8AA4EF}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{BDU-50LD}",
						[3] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[4] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[5] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[6] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[7] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[8] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[9] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[10] = "{BDU-50HD}",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{69DC8AE7-8F77-427B-B8AA-B19D3F478B66}",
						[3] = "LAU_88_AGM_65H_3",
						[4] = "{BDU-50LD}",
						[5] = "BRU-42_3*BDU-33",
						[6] = "LAU_117_AGM_65H",
						[7] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[8] = "{DAC53A2F-79CA-42FF-A77A-F5649B601308}",
						[9] = "LAU_88_AGM_65H_2_L",
						[10] = "{E6A6262A-CA08-4B3D-B030-E1A993B98452}",
						[11] = "LAU_117_AGM_65G",
						[12] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[13] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[14] = "{444BA8AE-82A7-4345-842E-76154EFCCA46}",
						[15] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[16] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[17] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[18] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[19] = "{BDU-50HD}",
						[20] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [3]
					[4] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{BDU-50LD}",
						[3] = "BRU-42_3*BDU-33",
						[4] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[5] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[6] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[7] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[8] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[9] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[10] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[11] = "{BDU-50HD}",
						[12] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [4]
					[5] = 
					{
						[1] = "{BDU-50LD}",
						[2] = "BRU-42_3*BDU-33",
						[3] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[4] = "{BDU-50HD}",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{BDU-50LD}",
						[2] = "BRU-42_3*BDU-33",
						[3] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[4] = "{BDU-50HD}",
					}, -- end of [6]
					[7] = 
					{
						[1] = "{BDU-50LD}",
						[2] = "BRU-42_3*BDU-33",
						[3] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[4] = "{BDU-50HD}",
					}, -- end of [7]
					[8] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{BDU-50LD}",
						[3] = "BRU-42_3*BDU-33",
						[4] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[5] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[6] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[7] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[8] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[9] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[10] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[11] = "{BDU-50HD}",
						[12] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [8]
					[9] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{69DC8AE7-8F77-427B-B8AA-B19D3F478B66}",
						[3] = "LAU_88_AGM_65H_2_R",
						[4] = "{E6A6262A-CA08-4B3D-B030-E1A993B98453}",
						[5] = "LAU_88_AGM_65H_3",
						[6] = "{BDU-50LD}",
						[7] = "BRU-42_3*BDU-33",
						[8] = "LAU_117_AGM_65H",
						[9] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[10] = "{DAC53A2F-79CA-42FF-A77A-F5649B601308}",
						[11] = "LAU_117_AGM_65G",
						[12] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[13] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[14] = "{444BA8AE-82A7-4345-842E-76154EFCCA46}",
						[15] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[16] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[17] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[18] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[19] = "{BDU-50HD}",
						[20] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [9]
					[10] = 
					{
						[1] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[2] = "{BDU-50LD}",
						[3] = "{6D6D5C07-2A90-4a68-9A74-C5D0CFFB05D9}",
						[4] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[5] = "{DDCE7D70-5313-4181-8977-F11018681662}",
						[6] = "{D22C2D63-E5C9-4247-94FB-5E8F3DE22B71}",
						[7] = "{1CA5E00B-D545-4ff9-9B53-5970E292F14D}",
						[8] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
						[9] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[10] = "{BDU-50HD}",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
					}, -- end of [10]
					[11] = 
					{
						[1] = "LAU-105_AIS_ASQ_T50_R",
						[2] = "{BDU-50LD}",
						[3] = "ALQ_184",
						[4] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
						[5] = "LAU-105_2*AIM-9P5",
						[6] = "{BDU-50HD}",
						[7] = "{6D21ECEA-F85B-4E8D-9D51-31DC9B8AA4EF}",
					}, -- end of [11]
				}, -- end of ["restricted"]
			}, -- end of ["payload"]
        }
    },
    ["AJS37"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{Robot24J}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{ARAKM70BHE}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{ARAKM70BHE}",
                    }, -- end of [3]
                    [5] =
                    {
                        ["CLSID"] = "{ARAKM70BHE}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{ARAKM70BHE}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{Robot24J}",
                    }, -- end of [7]
                }, -- end of ["pylons"]
                ["fuel"] = 4476,
                ["flare"] = 72,
                ["chaff"] = 210,
                ["gun"] = 100,
                ["restricted"] = 
                {
                    [3] = 
                    {
						[1] = "{RB75}",
						[2] = "{RB75B}",
						[3] = "{RB75T}",
						[4] = "{Robot74}",
                    }, -- end of [3]
                    [2] = 
                    {
						[1] = "{RB75}",
						[2] = "{RB75B}",
						[3] = "{Rb15}",
						[4] = "{Rb04AI}",
						[5] = "{RB75T}",
						[6] = "{Robot74}",
						[7] = "{Rb15AI}",
                    }, -- end of [2]
                    [5] = 
                    {
                        [1] = "{RB75}",
                        [2] = "{RB75B}",
                        [3] = "{RB75T}",
                        [4] = "{Robot74}",
                    }, -- end of [5]
                    [6] = 
                    {
                        [1] = "{RB75}",
                        [2] = "{RB75B}",
                        [3] = "{U22}",
                        [4] = "{Rb15}",
                        [5] = "{U22A}",
                        [6] = "{Rb04AI}",
                        [7] = "{RB75T}",
                        [8] = "{Robot74}",
                        [9] = "{Rb15AI}",
                    }, -- end of [6]
                }, -- end of ["restricted"]
            }, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 264,
                        [3] = 265,
                        [4] = 125,
                        [5] = 121,
                        [6] = 141,
                        [7] = 121.5,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
            -- ["livery_id"] = "37",
        }
    },
    ["AV8BNA"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["EWDispenserTBL"] = 2,
                ["EWDispenserBR"] = 2,
                ["AAR_Zone3"] = 0,
                ["AAR_Zone2"] = 0,
                ["EWDispenserTFR"] = 1,
                ["AAR_Zone1"] = 0,
                ["ClockTime"] = 1,
                ["RocketBurst"] = 1,
                ["LaserCode100"] = 6,
                ["LaserCode1"] = 8,
                ["EWDispenserTFL"] = 1,
                ["EWDispenserBL"] = 2,
                ["EWDispenserTBR"] = 2,
                ["LaserCode10"] = 8,
                ["MountNVG"] = false,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{A021F29D-18AB-4d3e-985C-FC9C60E35E9E}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{F3EFE0AB-E91A-42D8-9CA2-B63C91ED570A}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{GAU_12_Equalizer}",
                    }, -- end of [4]
                    [6] =
                    {
                        ["CLSID"] = "{F3EFE0AB-E91A-42D8-9CA2-B63C91ED570A}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{A021F29D-18AB-4d3e-985C-FC9C60E35E9E}",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
                    }, -- end of [8]
                }, -- end of ["pylons"]
                ["fuel"] = 3519.423,
                ["flare"] = 120,
                ["chaff"] = 60,
                ["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "CATM-9M",
						[2] = "{BDU-33}",
						[3] = "{AIS_ASQ_T50}",
						[4] = "{AGM_122_SIDEARM}",
						[5] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{BRU-42_2*GBU-38_LEFT}",
						[2] = "{F16A4DE0-116C-4A71-97F0-2CF85B0313EC}",
						[3] = "{BRU-42_2*GBU-12_LEFT}",
						[4] = "{BRU-42_3*GBU-38}",
						[5] = "{LAU_7_AGM_122_SIDEARM}",
						[6] = "{LAU-131 - 7 AGR-20 M282}",
						[7] = "{GBU_54_V_1B}",
						[8] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
						[9] = "BRU-42_3*BDU-33",
						[10] = "{GBU_32_V_2B}",
						[11] = "{BRU-70A_2*GBU-54_LEFT}",
						[12] = "LAU_117_AGM_65F",
						[13] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[14] = "{BDU-33}",
						[15] = "LAU_117_AGM_65L",
						[16] = "{LAU-131 - 7 AGR-20A}",
						[17] = "{BRU-42A_3*GBU-12}",
						[18] = "{GBU-38}",
						[19] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
						[20] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[21] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[22] = "{BRU-70A_3*GBU-54}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{BRU-42_2*GBU-38_LEFT}",
						[2] = "{F16A4DE0-116C-4A71-97F0-2CF85B0313EC}",
						[3] = "{BRU-42_2*GBU-12_LEFT}",
						[4] = "{LAU-131 - 7 AGR-20 M282}",
						[5] = "{GBU_54_V_1B}",
						[6] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
						[7] = "BRU-42_3*BDU-33",
						[8] = "{GBU_32_V_2B}",
						[9] = "{BRU-70A_2*GBU-54_LEFT}",
						[10] = "LAU_117_AGM_65F",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[12] = "{BDU-33}",
						[13] = "LAU_117_AGM_65L",
						[14] = "{LAU-131 - 7 AGR-20A}",
						[15] = "{GBU-38}",
						[16] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
						[17] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[18] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
					}, -- end of [3]
					[5] = 
					{
						[1] = "{ALQ_164_RF_Jammer}",
						[2] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{F16A4DE0-116C-4A71-97F0-2CF85B0313EC}",
						[2] = "{LAU-131 - 7 AGR-20 M282}",
						[3] = "{GBU_54_V_1B}",
						[4] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
						[5] = "BRU-42_3*BDU-33",
						[6] = "{GBU_32_V_2B}",
						[7] = "{BRU-42_2*GBU-38_RIGHT}",
						[8] = "LAU_117_AGM_65F",
						[9] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[10] = "{BDU-33}",
						[11] = "LAU_117_AGM_65L",
						[12] = "{LAU-131 - 7 AGR-20A}",
						[13] = "{BRU-70A_2*GBU-54_RIGHT}",
						[14] = "{GBU-38}",
						[15] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
						[16] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[17] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[18] = "{BRU-42_2*GBU-12_RIGHT}",
					}, -- end of [6]
					[7] = 
					{
						[1] = "{F16A4DE0-116C-4A71-97F0-2CF85B0313EC}",
						[2] = "{BRU-42_3*GBU-38}",
						[3] = "{LAU_7_AGM_122_SIDEARM}",
						[4] = "{LAU-131 - 7 AGR-20 M282}",
						[5] = "{GBU_54_V_1B}",
						[6] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
						[7] = "BRU-42_3*BDU-33",
						[8] = "{GBU_32_V_2B}",
						[9] = "{BRU-42_2*GBU-38_RIGHT}",
						[10] = "LAU_117_AGM_65F",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[12] = "{BDU-33}",
						[13] = "LAU_117_AGM_65L",
						[14] = "{LAU-131 - 7 AGR-20A}",
						[15] = "{BRU-42A_3*GBU-12}",
						[16] = "{BRU-70A_2*GBU-54_RIGHT}",
						[17] = "{GBU-38}",
						[18] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
						[19] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[20] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[21] = "{BRU-70A_3*GBU-54}",
						[22] = "{BRU-42_2*GBU-12_RIGHT}",
					}, -- end of [7]
					[8] = 
					{
						[1] = "CATM-9M",
						[2] = "{BDU-33}",
						[3] = "{AIS_ASQ_T50}",
						[4] = "{AGM_122_SIDEARM}",
						[5] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
					}, -- end of [8]
				}, -- end of ["restricted"]
            }, -- end of ["payload"]
            -- ["livery_id"] = "VMA-223D",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                        [9] = 0,
                        [10] = 0,
                        [11] = 0,
                        [12] = 0,
                        [13] = 0,
                        [14] = 0,
                        [15] = 0,
                        [16] = 0,
                        [17] = 0,
                        [18] = 0,
                        [19] = 0,
                        [20] = 0,
                        [21] = 0,
                        [22] = 0,
                        [23] = 0,
                        [24] = 0,
                        [25] = 0,
                        [26] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [3] = 265,
                        [4] = 256,
                        [5] = 254,
                        [6] = 250,
                        [7] = 270,
                        [8] = 257,
                        [9] = 255,
                        [10] = 262,
                        [11] = 259,
                        [12] = 268,
                        [13] = 269,
                        [14] = 260,
                        [15] = 263,
                        [16] = 261,
                        [17] = 267,
                        [18] = 251,
                        [19] = 253,
                        [20] = 266,
                        [21] = 133,
                        [22] = 257.8,
                        [23] = 122.1,
                        [24] = 123.3,
                        [25] = 344,
                        [26] = 385,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                        [9] = 0,
                        [10] = 0,
                        [11] = 0,
                        [12] = 0,
                        [13] = 0,
                        [14] = 0,
                        [15] = 0,
                        [16] = 0,
                        [17] = 0,
                        [18] = 0,
                        [19] = 0,
                        [20] = 0,
                        [21] = 0,
                        [22] = 0,
                        [23] = 0,
                        [24] = 0,
                        [25] = 0,
                        [26] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 257.8,
                        [3] = 122.1,
                        [4] = 123.3,
                        [5] = 344,
                        [6] = 385,
                        [7] = 130,
                        [8] = 385.4,
                        [9] = 139,
                        [10] = 140,
                        [11] = 134,
                        [12] = 132,
                        [13] = 131,
                        [14] = 129,
                        [15] = 138,
                        [16] = 121,
                        [17] = 126,
                        [18] = 125,
                        [19] = 128,
                        [20] = 122,
                        [21] = 123,
                        [22] = 124,
                        [23] = 135,
                        [24] = 136,
                        [25] = 141,
                        [26] = 127,
                    }, -- end of ["channels"]
                }, -- end of [2]
                [3] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                        [9] = 0,
                        [10] = 0,
                        [11] = 0,
                        [12] = 0,
                        [13] = 0,
                        [14] = 0,
                        [15] = 0,
                        [16] = 0,
                        [17] = 0,
                        [18] = 0,
                        [19] = 0,
                        [20] = 0,
                        [21] = 0,
                        [22] = 0,
                        [23] = 0,
                        [24] = 0,
                        [25] = 0,
                        [26] = 0,
                        [27] = 0,
                        [28] = 0,
                        [29] = 0,
                        [30] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 177,
                        [2] = 264,
                        [3] = 265,
                        [4] = 256,
                        [5] = 254,
                        [6] = 250,
                        [7] = 270,
                        [8] = 257,
                        [9] = 255,
                        [10] = 262,
                        [11] = 259,
                        [12] = 268,
                        [13] = 269,
                        [14] = 260,
                        [15] = 263,
                        [16] = 261,
                        [17] = 267,
                        [18] = 251,
                        [19] = 253,
                        [20] = 266,
                        [21] = 133,
                        [22] = 257.8,
                        [23] = 122.1,
                        [24] = 123.3,
                        [25] = 344,
                        [26] = 385,
                        [27] = 133,
                        [28] = 257.8,
                        [29] = 122.1,
                        [30] = 123.3,
                    }, -- end of ["channels"]
                }, -- end of [3]
            }, -- end of ["Radio"]
        }
    },
    ["C-101CC"]= {
        ["default"] = {
			["livery_id"] = "honduras - air force comayagua coronel jose enrique soto cano air base skin 2",
            ["AddPropAircraft"] =
            {
                ["SoloFlight"] = false,
                ["MountIFRHood"] = false,
                ["CameraRecorder"] = false,
                ["SightSunFilter"] = false,
                ["NetCrewControlPriority"] = 1,
                ["NS430allow"] = 0,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
			{
				["pylons"] = 
				{
					[1] = 
					{
						["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
					}, -- end of [1]
					[2] = 
					{
						["CLSID"] = "{BCE4E030-38E9-423E-98ED-24BE3DA87C32}",
					}, -- end of [2]
					[3] = 
					{
						["CLSID"] = "{BLG66_BELOUGA}",
					}, -- end of [3]
					[4] = 
					{
						["CLSID"] = "{AN-M3}",
					}, -- end of [4]
					[5] = 
					{
						["CLSID"] = "{BLG66_BELOUGA}",
					}, -- end of [5]
					[6] = 
					{
						["CLSID"] = "{BCE4E030-38E9-423E-98ED-24BE3DA87C32}",
					}, -- end of [6]
					[7] = 
					{
						["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
					}, -- end of [7]
				}, -- end of ["pylons"]
                ["fuel"] = 1329,
				["flare"] = 0,
				["chaff"] = 0,
				["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{FC23864E-3B80-48E3-9C03-4DA8B1D7497B}",
						[2] = "{6CEB49FC-DED8-4DED-B053-E1F033FF72D3}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[2] = "CBLS-200",
					}, -- end of [3]
					[5] = 
					{
						[1] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[2] = "CBLS-200",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
					}, -- end of [6]
					[7] = 
					{
						[1] = "{FC23864E-3B80-48E3-9C03-4DA8B1D7497B}",
						[2] = "{6CEB49FC-DED8-4DED-B053-E1F033FF72D3}",
					}, -- end of [7]
				}, -- end of ["restricted"]
			}, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [3] = 260,
                        [4] = 270,
                        [5] = 255,
                        [6] = 259,
                        [7] = 262,
                        [8] = 257,
                        [9] = 253,
                        [10] = 263,
                        [11] = 267,
                        [12] = 254,
                        [13] = 264,
                        [14] = 266,
                        [15] = 265,
                        [16] = 252,
                        [17] = 268,
                        [18] = 271,
                        [19] = 275,
                        [20] = 281,
                        [21] = 285,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["F-5E-3"]= {
        ["default"] = {
			["livery_id"] = "usaf 'southeast asia'",
			["AddPropAircraft"] = 
			{
				["LAU68ROF"] = 0,
				["ChaffSalvo"] = 0,
				["ChaffSalvoInt"] = 0,
				["LAU3ROF"] = 0,
				["ChaffBurst"] = 0,
				["LaserCode100"] = 6,
				["LaserCode1"] = 8,
				["ChaffBurstInt"] = 0,
				["FlareBurst"] = 0,
				["LaserCode10"] = 8,
				["FlareBurstInt"] = 0,
			}, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
                    }, -- end of [1]
                    [7] =
                    {
                        ["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
                    }, -- end of [7]
                    [4] =
                    {
                        ["CLSID"] = "{0395076D-2F77-4420-9D33-087A4398130B}",
                    }, -- end of [4]
                }, -- end of ["pylons"]
                ["fuel"] = 2046,
                ["flare"] = 15,
                ["ammo_type"] = 2,
                ["chaff"] = 30,
                ["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "CATM-9M",
						[2] = "{AIS_ASQ_T50}",
						[3] = "{AIM-9P5}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{BDU-33}",
						[2] = "LAU3_WP61",
						[3] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[4] = "LAU3_WP1B",
						[5] = "{BDU-50LGB}",
						[6] = "{BDU-50LD}",
						[7] = "{BDU-50HD}",
						[8] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[9] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[10] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[11] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{BDU-33}",
						[2] = "LAU3_WP61",
						[3] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[4] = "LAU3_WP1B",
						[5] = "{BDU-50LGB}",
						[6] = "{BDU-50LD}",
						[7] = "MXU-648-TP",
						[8] = "{BDU-50HD}",
						[9] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[10] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[12] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
					}, -- end of [3]
					[4] = 
					{
						[1] = "{BDU-33}",
						[2] = "{BDU-50LD}",
						[3] = "MXU-648-TP",
						[4] = "{BDU-50HD}",
					}, -- end of [4]
					[5] = 
					{
						[1] = "{BDU-33}",
						[2] = "LAU3_WP61",
						[3] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[4] = "LAU3_WP1B",
						[5] = "{BDU-50LGB}",
						[6] = "{BDU-50LD}",
						[7] = "MXU-648-TP",
						[8] = "{BDU-50HD}",
						[9] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[10] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[11] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[12] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{BDU-33}",
						[2] = "LAU3_WP61",
						[3] = "{65396399-9F5C-4ec3-A7D2-5A8F4C1D90C4}",
						[4] = "LAU3_WP1B",
						[5] = "{BDU-50LGB}",
						[6] = "{BDU-50LD}",
						[7] = "{BDU-50HD}",
						[8] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
						[9] = "{0877B74B-5A00-4e61-BA8A-A56450BA9E27}",
						[10] = "{1F7136CB-8120-4e77-B97B-945FF01FB67C}",
						[11] = "{FC85D2ED-501A-48ce-9863-49D468DDD5FC}",
					}, -- end of [6]
					[7] = 
					{
						[1] = "CATM-9M",
						[2] = "{AIS_ASQ_T50}",
						[3] = "{AIM-9P5}",
					}, -- end of [7]
				}, -- end of ["restricted"]
            }, -- end of ["payload"]
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [3] = 265,
                        [4] = 256,
                        [5] = 254,
                        [6] = 250,
                        [7] = 270,
                        [8] = 257,
                        [9] = 255,
                        [10] = 262,
                        [11] = 259,
                        [12] = 268,
                        [13] = 269,
                        [14] = 260,
                        [15] = 263,
                        [16] = 261,
                        [17] = 267,
                        [18] = 251,
                        [19] = 253,
                        [20] = 266,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["F-86F Sabre"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{HVARx2}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{HVARx2}",
                    }, -- end of [2]
                    [4] =
                    {
                        ["CLSID"] = "{F86ANM64}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{GAR-8}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{GAR-8}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{F86ANM64}",
                    }, -- end of [7]
                    [10] =
                    {
                        ["CLSID"] = "{HVARx2}",
                    }, -- end of [10]
                    [9] =
                    {
                        ["CLSID"] = "{HVARx2}",
                    }, -- end of [9]
                }, -- end of ["pylons"]
                ["fuel"] = 1282,
                ["flare"] = 0,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            -- ["livery_id"] = "iiaf bare metall",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [3] = 260,
                        [4] = 270,
                        [5] = 255,
                        [6] = 259,
                        [7] = 262,
                        [8] = 257,
                        [9] = 253,
                        [10] = 263,
                        [11] = 267,
                        [12] = 254,
                        [13] = 264,
                        [14] = 266,
                        [15] = 265,
                        [16] = 252,
                        [17] = 268,
                        [18] = 269,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["L-39ZA"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["DismountIFRHood"] = false,
                ["SoloFlight"] = false,
                ["NetCrewControlPriority"] = 1,
                ["NS430allow"] = false,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{R-3S}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{UB-16-57UMP}",
                    }, -- end of [2]
                    [4] =
                    {
                        ["CLSID"] = "{UB-16-57UMP}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{R-3S}",
                    }, -- end of [5]
                }, -- end of ["pylons"]
                ["fuel"] = 823.2,
                ["flare"] = 0,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            -- ["livery_id"] = "splinter camo desert",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                        [9] = 0,
                        [10] = 0,
                        [11] = 0,
                        [12] = 0,
                        [13] = 0,
                        [14] = 0,
                        [15] = 0,
                        [16] = 0,
                        [17] = 0,
                        [18] = 0,
                        [19] = 0,
                        [20] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 124,
                        [3] = 265,
                        [4] = 256,
                        [5] = 254,
                        [6] = 250,
                        [7] = 270,
                        [8] = 257,
                        [9] = 255,
                        [10] = 262,
                        [11] = 259,
                        [12] = 268,
                        [13] = 269,
                        [14] = 260,
                        [15] = 263,
                        [16] = 261,
                        [17] = 267,
                        [18] = 251,
                        [19] = 253,
                        [20] = 266,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["MiG-21Bis"]= {
        ["default"] = {
			["livery_id"] = "georgia (2)",
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{R-13M1}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{R-3R}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{PTB_800_MIG21}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{R-3R}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{R-13M1}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{ASO-2}",
                    }, -- end of [6]
                }, -- end of ["pylons"]
                ["fuel"] = 2280,
                ["flare"] = 40,
                ["ammo_type"] = 1,
                ["chaff"] = 18,
                ["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{R-60 2L}",
						[2] = "{R-60M}",
						[3] = "{R-60}",
						[4] = "{R-60M 2L}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{R-60 2L}",
						[2] = "{R-60M}",
						[3] = "{R-60}",
						[4] = "{R-60M 2L}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{RN-28}",
						[2] = "{RN-24}",
						[3] = "{SPS-141-100}",
					}, -- end of [3]
					[4] = 
					{
						[1] = "{R-60M}",
						[2] = "{R-60}",
						[3] = "{R-60 2R}",
						[4] = "{R-60M 2R}",
					}, -- end of [4]
					[5] = 
					{
						[1] = "{R-60M}",
						[2] = "{R-60}",
						[3] = "{R-60 2R}",
						[4] = "{R-60M 2R}",
					}, -- end of [5]
				}, -- end of ["restricted"]
            }, -- end of ["payload"]
            -- ["livery_id"] = "iraq - 17th sqn (1)",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [3] = 0,
                        [4] = 0,
                        [5] = 0,
                        [6] = 0,
                        [7] = 0,
                        [8] = 0,
                        [9] = 0,
                        [10] = 0,
                        [11] = 0,
                        [12] = 0,
                        [13] = 0,
                        [14] = 0,
                        [15] = 0,
                        [16] = 0,
                        [17] = 0,
                        [18] = 0,
                        [19] = 0,
                        [20] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 124,
                        [3] = 121,
                        [4] = 131,
                        [5] = 141,
                        [6] = 126,
                        [7] = 130,
                        [8] = 133,
                        [9] = 122,
                        [10] = 124,
                        [11] = 134,
                        [12] = 125,
                        [13] = 135,
                        [14] = 137,
                        [15] = 136,
                        [16] = 123,
                        [17] = 132,
                        [18] = 127,
                        [19] = 129,
                        [20] = 138,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["Su-25"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{R-3S}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{BD289E34-DF84-4C5E-9220-4B14C346E79D}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{3858707D-F5D5-4bbb-BDD8-ABB0530EBC7C}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{A0648264-4BC0-4EE8-A543-D119F6BA4257}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{F72F47E5-C83A-4B85-96ED-D3E46671EE9A}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{F72F47E5-C83A-4B85-96ED-D3E46671EE9A}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{A0648264-4BC0-4EE8-A543-D119F6BA4257}",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{3858707D-F5D5-4bbb-BDD8-ABB0530EBC7C}",
                    }, -- end of [8]
                    [9] =
                    {
                        ["CLSID"] = "{BD289E34-DF84-4C5E-9220-4B14C346E79D}",
                    }, -- end of [9]
                    [10] =
                    {
                        ["CLSID"] = "{R-3S}",
                    }, -- end of [10]
                }, -- end of ["pylons"]
                ["fuel"] = "2835",
                ["flare"] = 128,
                ["chaff"] = 128,
                ["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{682A481F-0CB5-4693-A382-D00DD4A156D7}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
					}, -- end of [3]
					[4] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
					}, -- end of [4]
					[5] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{D4A8D9B9-5C45-42e7-BBD2-0E54F8308432}",
					}, -- end of [5]
					[6] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{D4A8D9B9-5C45-42e7-BBD2-0E54F8308432}",
					}, -- end of [6]
					[7] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
					}, -- end of [7]
					[8] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
						[2] = "{F75187EF-1D9E-4DA9-84B4-1A1A14A3973A}",
						[3] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
					}, -- end of [8]
					[9] = 
					{
						[1] = "{0180F983-C14A-11d8-9897-000476191836}",
					}, -- end of [9]
					[10] = 
					{
						[1] = "{682A481F-0CB5-4693-A382-D00DD4A156D7}",
					}, -- end of [10]
				}, -- end of ["restricted"]
            }, -- end of ["payload"]
        }
    },
	["Mirage-F1CE"]= {
		["default"] = 
		{
			["livery_id"] = "Aerges Camo",
			["AddPropAircraft"] = 
			{
				["RocketSalvoF1"] = 1,
				["RadarCoverSettings"] = 1,
				["ChaffMultiTime"] = 1,
				["FlareMultiNumber"] = 1,
				["ChaffMultiNumber"] = 1,
				["LaserCode10"] = 8,
				["ChaffProgramNumber"] = 1,
				["LaserCode100"] = 6,
				["FlareMultiTime"] = 1,
				["ChaffProgramTime"] = 1,
				["RocketSalvoF4"] = 1,
				["LaserCode1"] = 8,
				["GunBurstSettings"] = 1,
				["INSStartMode"] = 1,
			}, -- end of ["AddPropAircraft"]
			["payload"] = 
			{
			["pylons"] = 
			{
				[1] = 
				{
					["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
				}, -- end of [1]
				[2] = 
				{
					["CLSID"] = "{BLG66_BELOUGA}",
				}, -- end of [2]
				[3] = 
				{
					["CLSID"] = "{BLG66_BELOUGA}",
				}, -- end of [3]
				[4] = 
				{
					["CLSID"] = "PTB-1200-F1",
				}, -- end of [4]
				[5] = 
				{
					["CLSID"] = "{BLG66_BELOUGA}",
				}, -- end of [5]
				[6] = 
				{
					["CLSID"] = "{BLG66_BELOUGA}",
				}, -- end of [6]
				[7] = 
				{
					["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
				}, -- end of [7]
			}, -- end of ["pylons"]
			["fuel"] = 3356,
			["flare"] = 15,
			["chaff"] = 30,
			["gun"] = 100,
			["restricted"] = 
			{
				[7] = 
				{
					[1] = "{AIM-9JULI}",
					[2] = "{FC23864E-3B80-48E3-9C03-4DA8B1D7497B}",
				}, -- end of [7]
				[3] = 
				{
					[1] = "{S530F}",
					[2] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
					[3] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
				}, -- end of [3]
				[1] = 
				{
					[1] = "{AIM-9JULI}",
					[2] = "{FC23864E-3B80-48E3-9C03-4DA8B1D7497B}",
				}, -- end of [1]
				[4] = 
				{
					[1] = "{51F9AAE5-964F-4D21-83FB-502E3BFE5F8A}",
					[2] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
				}, -- end of [4]
				[5] = 
				{
					[1] = "{S530F}",
					[2] = "{0D33DDAE-524F-4A4E-B5B8-621754FE3ADE}",
					[3] = "{DB769D48-67D7-42ED-A2BE-108D566C8B1E}",
				}, -- end of [5]
			}, -- end of ["restricted"]
		}, -- end of ["payload"]
			["Radio"] = 
			{
				[1] = 
				{
					["modulations"] = 
					{
					}, -- end of ["modulations"]
					["channels"] = 
					{
						[1] = 125,
						[2] = 119.25,
						[3] = 122,
						[4] = 126.5,
						[5] = 127,
						[6] = 129,
						[7] = 131,
						[8] = 133,
						[9] = 141,
						[10] = 250.5,
						[11] = 251,
						[12] = 253,
						[13] = 254,
						[14] = 257,
						[15] = 260,
						[16] = 261,
						[17] = 262,
						[18] = 263,
						[19] = 267,
						[20] = 270,
					}, -- end of ["channels"]
				}, -- end of [1]
				[2] = 
				{
					["modulations"] = 
					{
					}, -- end of ["modulations"]
					["channels"] = 
					{
						[1] = 265,
						[2] = 230,
						[3] = 240,
						[4] = 250.5,
						[5] = 251,
						[6] = 256,
						[7] = 257,
						[8] = 262,
						[9] = 263,
						[10] = 267,
						[11] = 270,
						[12] = 254,
						[13] = 264,
						[14] = 266,
						[15] = 265,
						[16] = 252,
						[17] = 268,
						[18] = 271,
						[19] = 275,
						[20] = 360,
					}, -- end of ["channels"]
				}, -- end of [2]
			}, -- end of ["Radio"]
		}
	},
    ["MiG-19P"]= {
        ["default"] = {
            ["livery_id"] = "poland 62 plm",
            ["AddPropAircraft"] = 
			{
				["MissileToneVolume"] = 5,
				["NAV_Initial_Hdg"] = 0,
				["ADF_Selected_Frequency"] = 1,
				["ADF_NEAR_Frequency"] = 303,
				["ADF_FAR_Frequency"] = 625,
				["MountSIRENA"] = false,
			}, -- end of ["AddPropAircraft"]
            ["payload"] = 
			{
				["pylons"] = 
				{
				}, -- end of ["pylons"]
				["fuel"] = 1800,
				["flare"] = 0,
				["ammo_type"] = 1,
				["chaff"] = 0,
				["gun"] = 100,
			}, -- end of ["payload"]
            ["Radio"] = 
			{
				[1] = 
				{
					["modulations"] = 
					{
						[6] = 0,
						[2] = 0,
						[3] = 0,
						[1] = 0,
						[4] = 0,
						[5] = 0,
					}, -- end of ["modulations"]
					["channels"] = 
					{
						[6] = 135,
						[2] = 124,
						[3] = 122,
						[1] = 121,
						[4] = 125,
						[5] = 127,
					}, -- end of ["channels"]
				}, -- end of [1]
			}, -- end of ["Radio"]
        }
    },
    ["MiG-15bis"]= {
        ["default"] = {
            ["livery_id"] = "polish_air force",
            ["payload"] = 
			{
				["pylons"] = 
				{
				}, -- end of ["pylons"]
				["fuel"] = 1172,
				["flare"] = 0,
				["chaff"] = 0,
				["gun"] = 100,
				["restricted"] = 
				{
					[1] = 
					{
						[1] = "{R-60 2L}",
						[2] = "{R-60M}",
						[3] = "{R-60}",
						[4] = "{R-60M 2L}",
					}, -- end of [1]
					[2] = 
					{
						[1] = "{R-60 2L}",
						[2] = "{R-60M}",
						[3] = "{R-60}",
						[4] = "{R-60M 2L}",
					}, -- end of [2]
					[3] = 
					{
						[1] = "{RN-28}",
						[2] = "{RN-24}",
						[3] = "{SPS-141-100}",
					}, -- end of [3]
					[4] = 
					{
						[1] = "{R-60M}",
						[2] = "{R-60}",
						[3] = "{R-60 2R}",
						[4] = "{R-60M 2R}",
					}, -- end of [4]
					[5] = 
					{
						[1] = "{R-60M}",
						[2] = "{R-60}",
						[3] = "{R-60 2R}",
						[4] = "{R-60M 2R}",
					}, -- end of [5]
				}, -- end of ["restricted"]
			}, -- end of ["payload"]
        }
    },
}