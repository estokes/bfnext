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
            ["livery_id"] = "Russia_VVS_Standard",
            ["AddPropAircraft"] = 
            {
                ["LeftEngineResource"] = 90,
                ["RightEngineResource"] = 90,
                ["PilotNVG"] = true,
                ["GunnersAISkill"] = 90,
                ["R60equipment"] = true,
                ["SimplifiedAI"] = false,
                ["OperatorNVG"] = true,
                ["ExhaustScreen"] = true,
                ["HideAngleBoxes"] = false,
                ["NetCrewControlPriority"] = 0,
                ["TrackAirTargets"] = true,
                ["NS430allow"] = true,
                ["R-60 equipment"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{2x9M120_Ataka_V}",
                    }, -- end of [1]
                    [2] = 
                    {
                        ["CLSID"] = "{B0DBC591-0F52-4F7D-AD7B-51E67725FB81}",
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
                        ["CLSID"] = "{2x9M120_Ataka_V}",
                    }, -- end of [5]
                    [6] = 
                    {
                        ["CLSID"] = "{2x9M120_Ataka_V}",
                    }, -- end of [6]
                }, -- end of ["pylons"]
                ["fuel"] = 1000,
                ["flare"] = 192,
                ["ammo_type"] = 1,
                ["chaff"] = 0,
                ["gun"] = 100,
                ["restricted"] = 
                {
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
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["Ka-50"]= {
        ["default"] = {
            ["livery_id"] = "Russia Standard Army",
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
                }, -- end of ["pylons"]
                ["fuel"] = "900",
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
    ["Mi-8MT"]= {
        ["default"] = {
            ["livery_id"] = "Russia_VVS_Standard",
            ["AddPropAircraft"] =
            {
                ["LeftEngineResource"] = 90,
                ["RightEngineResource"] = 90,
                ["NetCrewControlPriority"] = 1,
                ["ExhaustScreen"] = true,
                ["CargoHalfdoor"] = true,
                ["GunnersAISkill"] = 90,
                ["AdditionalArmor"] = true,
                ["NS430allow"] = true,
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
                ["fuel"] = "950",
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
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["AH-64D_BLK_II"]= {
        ["default"] = {
            ["AddPropAircraft"] = 
            {
                ["FlareProgramDelay"] = 0,
                ["FlareSalvoInterval"] = 0,
                ["NetCrewControlPriority"] = 0,
                ["CpgNVG"] = true,
                ["FCR_RFI_removed"] = true,
                ["FlareSalvoCount"] = 0,
                ["FlareBurstCount"] = 0,
                ["AIDisabled"] = false,
                ["FlareBurstInterval"] = 0,
                ["TrackAirTargets"] = true,
                ["PltNVG"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{88D18A5E-99C8-4B04-B40B-1C02F2018B6E}",
                    }, -- end of [1]
                    [2] = 
                    {
                        ["CLSID"] = "{88D18A5E-99C8-4B04-B40B-1C02F2018B6E}",
                    }, -- end of [2]
                    [3] = 
                    {
                        ["CLSID"] = "{88D18A5E-99C8-4B04-B40B-1C02F2018B6E}",
                    }, -- end of [3]
                    [4] = 
                    {
                        ["CLSID"] = "{88D18A5E-99C8-4B04-B40B-1C02F2018B6E}",
                    }, -- end of [4]
                }, -- end of ["pylons"]
                ["fuel"] = 863,
                ["flare"] = 60,
                ["ammo_type"] = 1,
                ["chaff"] = 30,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["Radio"] = 
            {
                [1] = 
                {
                    ["modulations"] = 
                    {
                        [7] = 0,
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [9] = 0,
                        [5] = 0,
                        [10] = 0,
                        [3] = 0,
                        [6] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [7] = 125,
                        [1] = 124,
                        [2] = 135,
                        [4] = 127,
                        [8] = 128,
                        [9] = 126,
                        [5] = 125,
                        [10] = 137,
                        [3] = 136,
                        [6] = 121,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] = 
                {
                    ["modulations"] = 
                    {
                        [7] = 0,
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [9] = 0,
                        [5] = 0,
                        [10] = 0,
                        [3] = 0,
                        [6] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [7] = 265,
                        [1] = 264,
                        [2] = 251,
                        [4] = 259,
                        [8] = 350,
                        [9] = 375,
                        [5] = 285,
                        [10] = 390,
                        [3] = 255,
                        [6] = 300,
                    }, -- end of ["channels"]
                }, -- end of [2]
                [4] = 
                {
                    ["modulations"] = 
                    {
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [7] = 30,
                        [1] = 35,
                        [2] = 30.01,
                        [4] = 30.02,
                        [8] = 30.04,
                        [9] = 30.045,
                        [5] = 30.025,
                        [10] = 30.05,
                        [3] = 30.015,
                        [6] = 30.03,
                    }, -- end of ["channels"]
                }, -- end of [4]
                [3] = 
                {
                    ["modulations"] = 
                    {
                    }, -- end of ["modulations"]
                    ["channels"] = 
                    {
                        [7] = 30,
                        [1] = 35,
                        [2] = 30.01,
                        [4] = 30.02,
                        [8] = 30.04,
                        [9] = 30.045,
                        [5] = 30.025,
                        [10] = 30.05,
                        [3] = 30.015,
                        [6] = 30.03,
                    }, -- end of ["channels"]
                }, -- end of [3]
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
                ["fuel"] = "322",
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
                        [3] = 251,
                        [4] = 259,
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
            ["livery_id"] = "US ARMY 1972",
        }
    },
    ["SA342M"]= {
        ["default"] = {
			
			["livery_id"] = "cyprus air force",
            ["AddPropAircraft"] =
            {
                ["RemoveTablet"] = false,
				["NS430allow"] = true,
            }, -- end of ["AddPropAircraft"]
			["payload"] = 
			{
				["pylons"] = 
				{
					[1] = 
					{
						["CLSID"] = "{HOT3_R2_M}",
					}, -- end of [1]
					[2] = 
					{
						["CLSID"] = "{HOT3_L2_M}",
					}, -- end of [2]
					[3] = 
					{
						["CLSID"] = "FAS}",
					}, -- end of [3]
					[4] = 
					{
						["CLSID"] = "{IR_Deflector}",
					}, -- end of [4]
				}, -- end of ["pylons"]
				["fuel"] = 204,
				["flare"] = 32,
				["chaff"] = 0,
				["gun"] = 100,
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
                        [8] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 30,
                        [2] = 35,
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
    ["SA342L"]= { --this one is for task ~= Escort which is for the non-mistral
        ["default"] = {
		
			["livery_id"] = "cyprus air force",
            ["AddPropAircraft"] =
            {
				["RemoveTablet"] = false,
                ["NS430allow"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
			{
				["pylons"] = 
				{
					[1] = 
					{
						["CLSID"] = "{GIAT_M621_APHE}",
					}, -- end of [1]
					[2] = 
					{
						["CLSID"] = "{TELSON8_SNEBT253}",
					}, -- end of [2]
					[4] = 
					{
						["CLSID"] = "{IR_Deflector}",
					}, -- end of [4]
				}, -- end of ["pylons"]
				["fuel"] = 204,
				["flare"] = 32,
				["chaff"] = 0,
				["gun"] = 100,
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
                        [8] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 30,
                        [2] = 35,
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
    ["SA342Minigun"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["NS430allow"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [6] =
                    {
                        ["CLSID"] = "{IR_Deflector}",
                    }, -- end of [6]
                    [5] =
                    {
                        ["CLSID"] = "{FAS}",
                    }, -- end of [5]
                }, -- end of ["pylons"]
                ["fuel"] = 350,
                ["flare"] = 32,
                ["chaff"] = 0,
                ["gun"] = 100,
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
                        [8] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 30,
                        [2] = 35,
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
    ["SA342Mistral"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["NS430allow"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{MBDA_MistralD}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{MBDA_MistralG}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{MBDA_MistralD}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{MBDA_MistralG}",
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
                ["fuel"] = 350,
                ["flare"] = 32,
                ["chaff"] = 0,
                ["gun"] = 100,
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
                        [8] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 30,
                        [2] = 35,
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
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "ALQ_184",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{319293F2-392C-4617-8315-7C88C22AF7C4}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "LAU_88_AGM_65H_3",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{60CC734F-0AFA-4E2E-82B8-93B941AB11CF}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
                    }, -- end of [5]
                    [7] =
                    {
                        ["CLSID"] = "{5335D97A-35A5-4643-9D9B-026C75961E52}",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{60CC734F-0AFA-4E2E-82B8-93B941AB11CF}",
                    }, -- end of [8]
                    [9] =
                    {
                        ["CLSID"] = "LAU_88_AGM_65H_3",
                    }, -- end of [9]
                    [10] =
                    {
                        ["CLSID"] = "{319293F2-392C-4617-8315-7C88C22AF7C4}",
                    }, -- end of [10]
                    [11] =
                    {
                        ["CLSID"] = "LAU-105_2*AIM-9P5",
                    }, -- end of [11]
                }, -- end of ["pylons"]
                ["fuel"] = 2515,
                ["flare"] = 120,
                ["ammo_type"] = 1,
                ["chaff"] = 240,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "23rd TFW England AFB (EL)",
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
                        ["CLSID"] = "{RB75}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{RB75}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{VIGGEN_X-TANK}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{RB75}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{RB75}",
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
                        [4] = 259,
                        [5] = 121,
                        [6] = 141,
                        [7] = 121.5,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
            ["livery_id"] = "37",
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
                ["MountNVG"] = true,
                ["EWDispenserTFL"] = 1,
                ["LaserCode100"] = 6,
                ["LaserCode1"] = 8,
                ["RocketBurst"] = 1,
                ["ClockTime"] = 1,
                ["EWDispenserBL"] = 2,
                ["LaserCode10"] = 8,
                ["EWDispenserTBR"] = 2,
            }, -- end of ["AddPropAircraft"]
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{AGM_122_SIDEARM}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "LAU_117_AGM_65G",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{LAU-131 - 7 AGR-20 M282}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{GAU_12_Equalizer}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{A111396E-D3E8-4b9c-8AC9-2432489304D5}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{LAU-131 - 7 AGR-20A}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "LAU_117_AGM_65G",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{6CEB49FC-DED8-4DED-B053-E1F033FF72D3}",
                    }, -- end of [8]
                }, -- end of ["pylons"]
                ["fuel"] = 3519.423,
                ["flare"] = 120,
                ["chaff"] = 60,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "VMA-223D",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [16] = 0,
                        [17] = 0,
                        [9] = 0,
                        [18] = 0,
                        [5] = 0,
                        [10] = 0,
                        [20] = 0,
                        [21] = 0,
                        [11] = 0,
                        [22] = 0,
                        [3] = 0,
                        [6] = 0,
                        [12] = 0,
                        [24] = 0,
                        [25] = 0,
                        [13] = 0,
                        [26] = 0,
                        [7] = 0,
                        [14] = 0,
                        [23] = 0,
                        [19] = 0,
                        [15] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 265,
                        [2] = 264,
                        [4] = 251,
                        [8] = 125,
                        [16] = 127.5,
                        [17] = 241,
                        [9] = 259,
                        [18] = 251,
                        [5] = 254,
                        [10] = 262,
                        [20] = 266,
                        [21] = 133,
                        [11] = 259,
                        [22] = 257.8,
                        [3] = 265,
                        [6] = 250,
                        [12] = 268,
                        [24] = 123.3,
                        [25] = 344,
                        [13] = 269,
                        [26] = 385,
                        [7] = 270,
                        [14] = 260,
                        [23] = 122.1,
                        [19] = 253,
                        [15] = 263,
                    }, -- end of ["channels"]
                }, -- end of [1]
                [2] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [16] = 0,
                        [17] = 0,
                        [9] = 0,
                        [18] = 0,
                        [5] = 0,
                        [10] = 0,
                        [20] = 0,
                        [21] = 0,
                        [11] = 0,
                        [22] = 0,
                        [3] = 0,
                        [6] = 0,
                        [12] = 0,
                        [24] = 0,
                        [25] = 0,
                        [13] = 0,
                        [26] = 0,
                        [7] = 0,
                        [14] = 0,
                        [23] = 0,
                        [19] = 0,
                        [15] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [4] = 264,
                        [8] = 251,
                        [16] = 127.5,
                        [17] = 241,
                        [9] = 259,
                        [18] = 125,
                        [5] = 344,
                        [10] = 140,
                        [20] = 122,
                        [21] = 123,
                        [11] = 134,
                        [22] = 124,
                        [3] = 122.1,
                        [6] = 385,
                        [12] = 132,
                        [24] = 136,
                        [25] = 141,
                        [13] = 131,
                        [26] = 127,
                        [7] = 130,
                        [14] = 129,
                        [23] = 135,
                        [19] = 128,
                        [15] = 138,
                    }, -- end of ["channels"]
                }, -- end of [2]
                [3] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [16] = 0,
                        [17] = 0,
                        [9] = 0,
                        [18] = 0,
                        [5] = 0,
                        [10] = 0,
                        [20] = 0,
                        [30] = 0,
                        [21] = 0,
                        [11] = 0,
                        [22] = 0,
                        [3] = 0,
                        [6] = 0,
                        [12] = 0,
                        [24] = 0,
                        [19] = 0,
                        [25] = 0,
                        [13] = 0,
                        [26] = 0,
                        [27] = 0,
                        [7] = 0,
                        [14] = 0,
                        [28] = 0,
                        [23] = 0,
                        [29] = 0,
                        [15] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 177,
                        [2] = 264,
                        [4] = 256,
                        [8] = 257,
                        [16] = 261,
                        [17] = 267,
                        [9] = 255,
                        [18] = 251,
                        [5] = 254,
                        [10] = 262,
                        [20] = 266,
                        [30] = 123.3,
                        [21] = 133,
                        [11] = 259,
                        [22] = 257.8,
                        [3] = 265,
                        [6] = 250,
                        [12] = 268,
                        [24] = 123.3,
                        [19] = 253,
                        [25] = 344,
                        [13] = 269,
                        [26] = 385,
                        [27] = 133,
                        [7] = 270,
                        [14] = 260,
                        [28] = 257.8,
                        [23] = 122.1,
                        [29] = 122.1,
                        [15] = 263,
                    }, -- end of ["channels"]
                }, -- end of [3]
            }, -- end of ["Radio"]
        }
    },
    ["C-101CC"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["SoloFlight"] = false,
                ["MountIFRHood"] = false,
                ["CameraRecorder"] = false,
                ["SightSunFilter"] = false,
                ["NetCrewControlPriority"] = 1,
                ["NS430allow"] = 1,
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
                        ["CLSID"] = "{C-101-DEFA553}",
                    }, -- end of [4]
                }, -- end of ["pylons"]
                ["fuel"] = 1257,
                ["flare"] = 0,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "USAF Agressor Fictional",
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
                        [4] = 270,
                        [8] = 257,
                        [16] = 252,
                        [17] = 268,
                        [9] = 253,
                        [18] = 271,
                        [5] = 255,
                        [10] = 263,
                        [20] = 281,
                        [21] = 285,
                        [11] = 267,
                        [3] = 260,
                        [6] = 259,
                        [12] = 254,
                        [13] = 264,
                        [7] = 262,
                        [14] = 266,
                        [19] = 275,
                        [15] = 265,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["F-14A-135-GR"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["LGB100"] = 6,
                ["M61BURST"] = 0,
                ["IlsChannel"] = 1,
                ["LGB1"] = 8,
                ["KY28Key"] = 1,
                ["TacanBand"] = 0,
                ["TacanChannel"] = 0,
                ["LGB1000"] = 1,
                ["LGB10"] = 8,
                ["INSAlignmentStored"] = true,
                ["UseLAU138"] = true,
                ["ALE39Loadout"] = 0,
            }, -- end of ["AddPropAircraft"]
            ["livery_id"] = "VF-154 Black Knights 101",
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
                        [3] = 127.5,
                        [4] = 241,
                        [5] = 259,
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
                        [19] = 268,
                        [20] = 269,
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
                        [27] = 0,
                        [28] = 0,
                        [29] = 0,
                        [30] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [3] = 264,
                        [4] = 127.5,
                        [5] = 241,
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
                        [19] = 268,
                        [20] = 269,
                        [21] = 225,
                        [22] = 258,
                        [23] = 260,
                        [24] = 270,
                        [25] = 255,
                        [26] = 259,
                        [27] = 262,
                        [28] = 257,
                        [29] = 253,
                        [30] = 263,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
        }
    },
    ["F-14B"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["LGB100"] = 6,
                ["M61BURST"] = 0,
                ["IlsChannel"] = 1,
                ["LGB1"] = 8,
                ["KY28Key"] = 1,
                ["TacanBand"] = 0,
                ["TacanChannel"] = 0,
                ["LGB1000"] = 1,
                ["LGB10"] = 8,
                ["INSAlignmentStored"] = true,
                ["UseLAU138"] = true,
                ["ALE39Loadout"] = 0,
            }, -- end of ["AddPropAircraft"]
            ["livery_id"] = "VF-102 Diamondbacks",
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
                        [3] = 125,
                        [4] = 251,
                        [5] = 127.5,
                        [6] = 241,
                        [7] = 259,
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
                        [19] = 268,
                        [20] = 269,
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
                        [27] = 0,
                        [28] = 0,
                        [29] = 0,
                        [30] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [3] = 264,
                        [4] = 251,
                        [5] = 127.5,
                        [6] = 241,
                        [7] = 259,
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
                        [19] = 268,
                        [20] = 269,
                        [21] = 225,
                        [22] = 258,
                        [23] = 260,
                        [24] = 270,
                        [25] = 255,
                        [26] = 259,
                        [27] = 262,
                        [28] = 257,
                        [29] = 253,
                        [30] = 263,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
            ["payload"] = 
            {
                ["pylons"] = 
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 60,
                ["ammo_type"] = 1,
                ["chaff"] = 140,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["F-16C_50"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["LaserCode100"] = 6,
                ["LaserCode1"] = 8,
                ["HelmetMountedDevice"] = 1,
                ["LaserCode10"] = 8,
                ["LAU3ROF"] = 0,
            }, -- end of ["AddPropAircraft"]
            ["livery_id"] = "default",
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
                        [1] = 265,
                        [2] = 264,
                        [3] = 251,
                        [4] = 240,
                        [5] = 259,
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
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 135,
                        [3] = 136,
                        [4] = 127,
                        [5] = 125,
                        [6] = 121,
                        [7] = 141,
                        [8] = 128,
                        [9] = 126,
                        [10] = 133,
                        [11] = 130,
                        [12] = 139,
                        [13] = 140,
                        [14] = 131,
                        [15] = 134,
                        [16] = 132,
                        [17] = 138,
                        [18] = 122,
                        [19] = 124,
                        [20] = 137,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
            ["payload"] =
            {
                ["pylons"] =
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 32,
                ["chaff"] = 36,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["FA-18C_hornet"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["OuterBoard"] = 0,
                ["InnerBoard"] = 0,
                ["HelmetMountedDevice"] = 1,
            }, -- end of ["AddPropAircraft"]
            ["livery_id"] = "VFA-37",
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
                        [1] = 265,
                        [2] = 264,
                        [3] = 125,
                        [4] = 251,
                        [5] = 127.5,
                        [6] = 241,
                        [7] = 259,
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
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [3] = 264,
                        [4] = 251,
                        [5] = 127.5,
                        [6] = 241,
                        [7] = 259,
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
                }, -- end of [2]
            }, -- end of ["Radio"]
            ["payload"] =
            {
                ["pylons"] =
                {
                }, -- end of ["pylons"]
                ["ammo_type"] = 3,
                ["fuel"] = 0,
                ["flare"] = 40,
                ["chaff"] = 80,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["JF-17"]= {
        ["default"] = {
            ["livery_id"] = "'Splinter' Camo for Blue Side (Fictional)",
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
                        [1] = 125.5,
                        [2] = 264.5,
                        [4] = 242.5,
                        [8] = 246.5,
                        [16] = 115.5,
                        [17] = 116,
                        [9] = 247.5,
                        [18] = 116.5,
                        [5] = 243.5,
                        [10] = 259.5,
                        [20] = 117.5,
                        [11] = 30.5,
                        [3] = 265.5,
                        [6] = 244.5,
                        [12] = 35.5,
                        [13] = 30,
                        [7] = 245.5,
                        [14] = 35,
                        [15] = 115,
                        [19] = 117,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
            ["payload"] =
            {
                ["pylons"] =
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 32,
                ["chaff"] = 36,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["J-11A"]= {
        ["default"] = {
            ["livery_id"] = "PLAAF 14th AD",
            ["payload"] = 
            {
                ["pylons"] = 
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 96,
                ["chaff"] = 96,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["M-2000C"]= {
        ["default"] = {
            ["AddPropAircraft"] =
            {
                ["ReadyALCM"] = true,
                ["GunBurst"] = 1,
                ["ForceINSRules"] = true,
                ["LaserCode100"] = 6,
                ["RocketBurst"] = 6,
                ["DisableVTBExport"] = false,
                ["NoDDMSensor"] = true,
                ["LaserCode1"] = 8,
                ["WpBullseye"] = 0,
                ["LoadNVGCase"] = true,
                ["InitHotDrift"] = 0,
                ["LaserCode10"] = 8,
                ["EnableTAF"] = true,
            }, -- end of ["AddPropAircraft"]
            ["livery_id"] = "AdA Alsace LF-2",
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
                        [1] = 265,
                        [2] = 264,
                        [3] = 251,
                        [4] = 241,
                        [5] = 259,
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
                        [18] = 252,
                        [19] = 253,
                        [20] = 266,
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
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 135,
                        [3] = 136,
                        [4] = 127,
                        [5] = 125,
                        [6] = 121,
                        [7] = 141,
                        [8] = 128,
                        [9] = 126,
                        [10] = 133,
                        [11] = 130,
                        [12] = 139,
                        [13] = 140,
                        [14] = 131,
                        [15] = 134,
                        [16] = 132,
                        [17] = 138,
                        [18] = 122,
                        [19] = 124,
                        [20] = 137,
                    }, -- end of ["channels"]
                }, -- end of [2]
            }, -- end of ["Radio"]
			["payload"] = 
            {
                ["pylons"] = 
                {
                    [2] = 
                    {
                        ["CLSID"] = "{Matra_S530D}",
                    }, -- end of [2]
                    [8] = 
                    {
                        ["CLSID"] = "{Matra_S530D}",
                    }, -- end of [8]
                    [1] = 
                    {
                        ["CLSID"] = "{MMagicII}",
                    }, -- end of [1]
                    [5] = 
                    {
                        ["CLSID"] = "{M2KC_RPL_522}",
                    }, -- end of [5]
                    [9] = 
                    {
                        ["CLSID"] = "{MMagicII}",
                    }, -- end of [9]
                }, -- end of ["pylons"]
            ["fuel"] = 3165,
            ["flare"] = 64,
            ["ammo_type"] = 1,
            ["chaff"] = 234,
            ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["F-5E-3"]= {
        ["default"] = {
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{AIM-9P5}",
                    }, -- end of [1]
                    [7] = 
                    {
                        ["CLSID"] = "{AIM-9P5}",
                    }, -- end of [7]
                    [4] = 
                    {
                        ["CLSID"] = "{PTB-150GAL}",
                    }, -- end of [4]
                }, -- end of ["pylons"]
                ["fuel"] = 2046,
                ["flare"] = 15,
                ["ammo_type"] = 2,
                ["chaff"] = 30,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "IR IRIAF Camo",
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
                        [4] = 251,
                        [8] = 259,
                        [16] = 261,
                        [17] = 267,
                        [9] = 255,
                        [18] = 251,
                        [5] = 254,
                        [10] = 262,
                        [20] = 266,
                        [11] = 259,
                        [3] = 265,
                        [6] = 250,
                        [12] = 268,
                        [13] = 269,
                        [7] = 270,
                        [14] = 260,
                        [19] = 253,
                        [15] = 263,
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
            ["livery_id"] = "iiaf bare metall",
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
                        [3] = 251,
                        [4] = 259,
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
                ["NS430allow"] = true,
            }, -- end of ["AddPropAircraft"]
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{APU-60-1_R_60M}",
                    }, -- end of [1]
                    [5] = 
                    {
                        ["CLSID"] = "{APU-60-1_R_60M}",
                    }, -- end of [5]
                }, -- end of ["pylons"]
                ["fuel"] = 588,
                ["flare"] = 0,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "splinter camo desert",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [16] = 0,
                        [17] = 0,
                        [9] = 0,
                        [18] = 0,
                        [5] = 0,
                        [10] = 0,
                        [20] = 0,
                        [11] = 0,
                        [3] = 0,
                        [6] = 0,
                        [12] = 0,
                        [13] = 0,
                        [7] = 0,
                        [14] = 0,
                        [19] = 0,
                        [15] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [4] = 264,
                        [8] = 251,
                        [16] = 259,
                        [17] = 267,
                        [9] = 255,
                        [18] = 251,
                        [5] = 254,
                        [10] = 262,
                        [20] = 266,
                        [11] = 259,
                        [3] = 265,
                        [6] = 250,
                        [12] = 268,
                        [13] = 269,
                        [7] = 270,
                        [14] = 260,
                        [19] = 253,
                        [15] = 263,
                    }, -- end of ["channels"]
                }, -- end of [1]
            }, -- end of ["Radio"]
        }
    },
    ["MiG-29A"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{E8069896-8435-4B90-95C0-01A03AE6E400}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{2BEC576B-CDF5-4B7F-961F-B0FA4312B841}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{E8069896-8435-4B90-95C0-01A03AE6E400}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [7]
                }, -- end of ["pylons"]
                ["fuel"] = "3376",
                ["flare"] = 30,
                ["chaff"] = 30,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["MiG-29S"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{B4C01D60-A8A3-4237-BD72-CA7655BC0FE9}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{B4C01D60-A8A3-4237-BD72-CA7655BC0FE9}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{2BEC576B-CDF5-4B7F-961F-B0FA4312B841}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{B4C01D60-A8A3-4237-BD72-CA7655BC0FE9}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{B4C01D60-A8A3-4237-BD72-CA7655BC0FE9}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{FBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [7]
                }, -- end of ["pylons"]
                ["fuel"] = "0",
                ["flare"] = 30,
                ["chaff"] = 30,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["Su-25T"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{44EE8698-89F9-48EE-AF36-5FD31896A82D}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{CBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{752AF1D2-EBCC-4bd7-A1E7-2357F5601C70}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{F789E86A-EE2E-4E6B-B81E-D5E5F903B6ED}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{601C99F7-9AF3-4ed7-A565-F8B8EC0D7AAC}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{0519A264-0AB6-11d6-9193-00A0249B6F00}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{601C99F7-9AF3-4ed7-A565-F8B8EC0D7AAC}",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{F789E86A-EE2E-4E6B-B81E-D5E5F903B6ED}",
                    }, -- end of [8]
                    [9] =
                    {
                        ["CLSID"] = "{752AF1D2-EBCC-4bd7-A1E7-2357F5601C70}",
                    }, -- end of [9]
                    [10] =
                    {
                        ["CLSID"] = "{CBC29BFE-3D24-4C64-B81D-941239D12249}",
                    }, -- end of [10]
                    [11] =
                    {
                        ["CLSID"] = "{44EE8698-89F9-48EE-AF36-5FD31896A82C}",
                    }, -- end of [11]
                }, -- end of ["pylons"]
                ["fuel"] = "3790",
                ["flare"] = 128,
                ["chaff"] = 128,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "af standard 1",
        }
    },
    ["MiG-21Bis"]= {
        ["default"] = {
            ["payload"] =
            {
                ["pylons"] =
                {
                    [1] =
                    {
                        ["CLSID"] = "{R-60M 2L}",
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
                        ["CLSID"] = "{R-60M 2R}",
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
            }, -- end of ["payload"]
            ["livery_id"] = "iraq - 17th sqn (1)",
            ["Radio"] =
            {
                [1] =
                {
                    ["modulations"] =
                    {
                        [1] = 0,
                        [2] = 0,
                        [4] = 0,
                        [8] = 0,
                        [16] = 0,
                        [17] = 0,
                        [9] = 0,
                        [18] = 0,
                        [5] = 0,
                        [10] = 0,
                        [20] = 0,
                        [11] = 0,
                        [3] = 0,
                        [6] = 0,
                        [12] = 0,
                        [13] = 0,
                        [7] = 0,
                        [14] = 0,
                        [19] = 0,
                        [15] = 0,
                    }, -- end of ["modulations"]
                    ["channels"] =
                    {
                        [1] = 125,
                        [2] = 265,
                        [4] = 264,
                        [8] = 251,
                        [16] = 259,
                        [17] = 132,
                        [9] = 122,
                        [18] = 127,
                        [5] = 141,
                        [10] = 124,
                        [20] = 138,
                        [11] = 134,
                        [3] = 121,
                        [6] = 126,
                        [12] = 125,
                        [13] = 135,
                        [7] = 130,
                        [14] = 137,
                        [19] = 129,
                        [15] = 136,
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
                        ["CLSID"] = "{682A481F-0CB5-4693-A382-D00DD4A156D7}",
                    }, -- end of [1]
                    [2] =
                    {
                        ["CLSID"] = "{F72F47E5-C83A-4B85-96ED-D3E46671EE9A}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{D5435F26-F120-4FA3-9867-34ACE562EF1B}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{0180F983-C14A-11d8-9897-000476191836}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{0180F983-C14A-11d8-9897-000476191836}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{79D73885-0801-45a9-917F-C90FE1CE3DFC}",
                    }, -- end of [7]
                    [8] =
                    {
                        ["CLSID"] = "{D5435F26-F120-4FA3-9867-34ACE562EF1B}",
                    }, -- end of [8]
                    [9] =
                    {
                        ["CLSID"] = "{F72F47E5-C83A-4B85-96ED-D3E46671EE9A}",
                    }, -- end of [9]
                    [10] =
                    {
                        ["CLSID"] = "{682A481F-0CB5-4693-A382-D00DD4A156D7}",
                    }, -- end of [10]
                }, -- end of ["pylons"]
                ["fuel"] = "2835",
                ["flare"] = 128,
                ["chaff"] = 128,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["livery_id"] = "Combat",
    ["Mirage-F1CE"]= {
        ["default"] = {
            ["payload"] = 
            {
                ["pylons"] = 
                {
                    [1] = 
                    {
                        ["CLSID"] = "{AIM-9JULI}",
                    }, -- end of [1]
                    [7] = 
                    {
                        ["CLSID"] = "{AIM-9JULI}",
                    }, -- end of [7]
                    [4] = 
                    {
                        ["CLSID"] = "{R530F_EM}",
                    }, -- end of [4]
                }, -- end of ["pylons"]
                ["fuel"] = 3356,
                ["flare"] = 15,
                ["chaff"] = 30,
                ["gun"] = 100,
            }, -- end of ["payload"]
            ["livery_id"] = "Aerges Camo",
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
    ["F-15C"]= {
        ["default"] = {
            ["livery_id"] = "12th fighter sqn (ak)",
            ["payload"] = 
            {
                ["pylons"] = 
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 60,
                ["chaff"] = 120,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },
    ["Su-27"]= {
        ["default"] = {
            ["livery_id"] = "Air Force Standard old",
            ["payload"] = 
            {
                ["pylons"] = 
                {
                }, -- end of ["pylons"]
                ["fuel"] = 0,
                ["flare"] = 96,
                ["chaff"] = 96,
                ["gun"] = 100,
            }, -- end of ["payload"]
        }
    },

}
   