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
            -- ["livery_id"] = "Russia_VVS_Standard",
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
                        ["CLSID"] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
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
                        ["CLSID"] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{B919B0F4-7C25-455E-9A02-CEA51DB895E3}",
                    }, -- end of [6]
                }, -- end of ["pylons"]
                ["fuel"] = 1001,
                ["flare"] = 192,
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
                ["fuel"] = 5029,
                ["flare"] = 120,
                ["ammo_type"] = 1,
                ["chaff"] = 240,
                ["gun"] = 100,
            }, -- end of ["payload"]
            -- ["livery_id"] = "104th fs maryland ang, baltimore (md)",
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
                        ["CLSID"] = "{FD90A1DC-9147-49FA-BF56-CB83EF0BD32B}",
                    }, -- end of [2]
                    [3] =
                    {
                        ["CLSID"] = "{BCE4E030-38E9-423E-98ED-24BE3DA87C32}",
                    }, -- end of [3]
                    [4] =
                    {
                        ["CLSID"] = "{AN-M3}",
                    }, -- end of [4]
                    [5] =
                    {
                        ["CLSID"] = "{BCE4E030-38E9-423E-98ED-24BE3DA87C32}",
                    }, -- end of [5]
                    [6] =
                    {
                        ["CLSID"] = "{FD90A1DC-9147-49FA-BF56-CB83EF0BD32B}",
                    }, -- end of [6]
                    [7] =
                    {
                        ["CLSID"] = "{9BFD8C90-F7AE-4e90-833B-BFD0CED0E536}",
                    }, -- end of [7]
                }, -- end of ["pylons"]
                ["fuel"] = 1796,
                ["flare"] = 0,
                ["chaff"] = 0,
                ["gun"] = 100,
            }, -- end of ["payload"]
            -- ["livery_id"] = "USAF Agressor Fictional",
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
            }, -- end of ["payload"]
            -- ["livery_id"] = "IR IRIAF Camo",
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
            }, -- end of ["payload"]
        }
    },
    -- ["livery_id"] = "Combat",
}