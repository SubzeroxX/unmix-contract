/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/un_mix.json`.
 */
export type UnMix = {
    "address": "EJJWDU26jjbCdY9LU1SkaKUttDw9DjMP9p7ACK4CGJ8z",
    "metadata": {
        "name": "unMix",
        "version": "0.1.0",
        "spec": "0.1.0",
        "description": "Created with Anchor"
    },
    "instructions": [
        {
            "name": "enableLevel",
            "discriminator": [
                159,
                5,
                208,
                55,
                199,
                191,
                189,
                156
            ],
            "accounts": [
                {
                    "name": "admin",
                    "signer": true
                },
                {
                    "name": "global",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    103,
                                    108,
                                    111,
                                    98,
                                    97,
                                    108
                                ]
                            }
                        ]
                    }
                }
            ],
            "args": [
                {
                    "name": "level",
                    "type": "u8"
                },
                {
                    "name": "enabled",
                    "type": "bool"
                }
            ]
        },
        {
            "name": "fund",
            "discriminator": [
                218,
                188,
                111,
                221,
                152,
                113,
                174,
                7
            ],
            "accounts": [
                {
                    "name": "user",
                    "writable": true,
                    "signer": true
                },
                {
                    "name": "global",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    103,
                                    108,
                                    111,
                                    98,
                                    97,
                                    108
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "userFund",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    117,
                                    115,
                                    101,
                                    114,
                                    95,
                                    102,
                                    117,
                                    110,
                                    100
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "user"
                            }
                        ]
                    }
                },
                {
                    "name": "usdcMint"
                },
                {
                    "name": "userUsdc",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "account",
                                "path": "user"
                            },
                            {
                                "kind": "const",
                                "value": [
                                    6,
                                    221,
                                    246,
                                    225,
                                    215,
                                    101,
                                    161,
                                    147,
                                    217,
                                    203,
                                    225,
                                    70,
                                    206,
                                    235,
                                    121,
                                    172,
                                    28,
                                    180,
                                    133,
                                    237,
                                    95,
                                    91,
                                    55,
                                    145,
                                    58,
                                    140,
                                    245,
                                    133,
                                    126,
                                    255,
                                    0,
                                    169
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "usdcMint"
                            }
                        ],
                        "program": {
                            "kind": "const",
                            "value": [
                                140,
                                151,
                                37,
                                143,
                                78,
                                36,
                                137,
                                241,
                                187,
                                61,
                                16,
                                41,
                                20,
                                142,
                                13,
                                131,
                                11,
                                90,
                                19,
                                153,
                                218,
                                255,
                                16,
                                132,
                                4,
                                142,
                                123,
                                216,
                                219,
                                233,
                                248,
                                89
                            ]
                        }
                    }
                },
                {
                    "name": "vault",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "account",
                                "path": "authority"
                            },
                            {
                                "kind": "const",
                                "value": [
                                    6,
                                    221,
                                    246,
                                    225,
                                    215,
                                    101,
                                    161,
                                    147,
                                    217,
                                    203,
                                    225,
                                    70,
                                    206,
                                    235,
                                    121,
                                    172,
                                    28,
                                    180,
                                    133,
                                    237,
                                    95,
                                    91,
                                    55,
                                    145,
                                    58,
                                    140,
                                    245,
                                    133,
                                    126,
                                    255,
                                    0,
                                    169
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "usdcMint"
                            }
                        ],
                        "program": {
                            "kind": "const",
                            "value": [
                                140,
                                151,
                                37,
                                143,
                                78,
                                36,
                                137,
                                241,
                                187,
                                61,
                                16,
                                41,
                                20,
                                142,
                                13,
                                131,
                                11,
                                90,
                                19,
                                153,
                                218,
                                255,
                                16,
                                132,
                                4,
                                142,
                                123,
                                216,
                                219,
                                233,
                                248,
                                89
                            ]
                        }
                    }
                },
                {
                    "name": "authority",
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    97,
                                    117,
                                    116,
                                    104,
                                    111,
                                    114,
                                    105,
                                    116,
                                    121
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "tokenProgram",
                    "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                },
                {
                    "name": "systemProgram",
                    "address": "11111111111111111111111111111111"
                }
            ],
            "args": [
                {
                    "name": "level",
                    "type": "u8"
                },
                {
                    "name": "amount",
                    "type": "u64"
                }
            ]
        },
        {
            "name": "initialize",
            "discriminator": [
                175,
                175,
                109,
                31,
                13,
                152,
                155,
                237
            ],
            "accounts": [
                {
                    "name": "admin",
                    "writable": true,
                    "signer": true
                },
                {
                    "name": "usdcMint"
                },
                {
                    "name": "global",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    103,
                                    108,
                                    111,
                                    98,
                                    97,
                                    108
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "authority",
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    97,
                                    117,
                                    116,
                                    104,
                                    111,
                                    114,
                                    105,
                                    116,
                                    121
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "vault",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "account",
                                "path": "authority"
                            },
                            {
                                "kind": "const",
                                "value": [
                                    6,
                                    221,
                                    246,
                                    225,
                                    215,
                                    101,
                                    161,
                                    147,
                                    217,
                                    203,
                                    225,
                                    70,
                                    206,
                                    235,
                                    121,
                                    172,
                                    28,
                                    180,
                                    133,
                                    237,
                                    95,
                                    91,
                                    55,
                                    145,
                                    58,
                                    140,
                                    245,
                                    133,
                                    126,
                                    255,
                                    0,
                                    169
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "usdcMint"
                            }
                        ],
                        "program": {
                            "kind": "const",
                            "value": [
                                140,
                                151,
                                37,
                                143,
                                78,
                                36,
                                137,
                                241,
                                187,
                                61,
                                16,
                                41,
                                20,
                                142,
                                13,
                                131,
                                11,
                                90,
                                19,
                                153,
                                218,
                                255,
                                16,
                                132,
                                4,
                                142,
                                123,
                                216,
                                219,
                                233,
                                248,
                                89
                            ]
                        }
                    }
                },
                {
                    "name": "tokenProgram",
                    "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                },
                {
                    "name": "associatedTokenProgram",
                    "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
                },
                {
                    "name": "systemProgram",
                    "address": "11111111111111111111111111111111"
                }
            ],
            "args": []
        },
        {
            "name": "withdraw",
            "discriminator": [
                183,
                18,
                70,
                156,
                148,
                109,
                161,
                34
            ],
            "accounts": [
                {
                    "name": "admin",
                    "signer": true
                },
                {
                    "name": "global",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    103,
                                    108,
                                    111,
                                    98,
                                    97,
                                    108
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "usdcMint"
                },
                {
                    "name": "vault",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "account",
                                "path": "authority"
                            },
                            {
                                "kind": "const",
                                "value": [
                                    6,
                                    221,
                                    246,
                                    225,
                                    215,
                                    101,
                                    161,
                                    147,
                                    217,
                                    203,
                                    225,
                                    70,
                                    206,
                                    235,
                                    121,
                                    172,
                                    28,
                                    180,
                                    133,
                                    237,
                                    95,
                                    91,
                                    55,
                                    145,
                                    58,
                                    140,
                                    245,
                                    133,
                                    126,
                                    255,
                                    0,
                                    169
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "usdcMint"
                            }
                        ],
                        "program": {
                            "kind": "const",
                            "value": [
                                140,
                                151,
                                37,
                                143,
                                78,
                                36,
                                137,
                                241,
                                187,
                                61,
                                16,
                                41,
                                20,
                                142,
                                13,
                                131,
                                11,
                                90,
                                19,
                                153,
                                218,
                                255,
                                16,
                                132,
                                4,
                                142,
                                123,
                                216,
                                219,
                                233,
                                248,
                                89
                            ]
                        }
                    }
                },
                {
                    "name": "adminUsdc",
                    "writable": true,
                    "pda": {
                        "seeds": [
                            {
                                "kind": "account",
                                "path": "admin"
                            },
                            {
                                "kind": "const",
                                "value": [
                                    6,
                                    221,
                                    246,
                                    225,
                                    215,
                                    101,
                                    161,
                                    147,
                                    217,
                                    203,
                                    225,
                                    70,
                                    206,
                                    235,
                                    121,
                                    172,
                                    28,
                                    180,
                                    133,
                                    237,
                                    95,
                                    91,
                                    55,
                                    145,
                                    58,
                                    140,
                                    245,
                                    133,
                                    126,
                                    255,
                                    0,
                                    169
                                ]
                            },
                            {
                                "kind": "account",
                                "path": "usdcMint"
                            }
                        ],
                        "program": {
                            "kind": "const",
                            "value": [
                                140,
                                151,
                                37,
                                143,
                                78,
                                36,
                                137,
                                241,
                                187,
                                61,
                                16,
                                41,
                                20,
                                142,
                                13,
                                131,
                                11,
                                90,
                                19,
                                153,
                                218,
                                255,
                                16,
                                132,
                                4,
                                142,
                                123,
                                216,
                                219,
                                233,
                                248,
                                89
                            ]
                        }
                    }
                },
                {
                    "name": "authority",
                    "pda": {
                        "seeds": [
                            {
                                "kind": "const",
                                "value": [
                                    97,
                                    117,
                                    116,
                                    104,
                                    111,
                                    114,
                                    105,
                                    116,
                                    121
                                ]
                            }
                        ]
                    }
                },
                {
                    "name": "tokenProgram",
                    "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                }
            ],
            "args": [
                {
                    "name": "amount",
                    "type": "u64"
                }
            ]
        }
    ],
    "accounts": [
        {
            "name": "global",
            "discriminator": [
                167,
                232,
                232,
                177,
                200,
                108,
                114,
                127
            ]
        },
        {
            "name": "userFund",
            "discriminator": [
                11,
                55,
                142,
                181,
                228,
                65,
                176,
                30
            ]
        }
    ],
    "errors": [
        {
            "code": 6000,
            "name": "invalidLevel",
            "msg": "Invalid level."
        },
        {
            "code": 6001,
            "name": "insufficientFunds",
            "msg": "Insuffitient funds"
        },
        {
            "code": 6002,
            "name": "levelNotEnabled",
            "msg": "Level not enabled."
        },
        {
            "code": 6003,
            "name": "amountTooSmall",
            "msg": "Amount too small."
        },
        {
            "code": 6004,
            "name": "exceedsMaxPerAccount",
            "msg": "Exceeds max per account for level."
        },
        {
            "code": 6005,
            "name": "exceedsLevelTotal",
            "msg": "Exceeds total for level."
        },
        {
            "code": 6006,
            "name": "mintMismatch",
            "msg": "Mint mismatch."
        },
        {
            "code": 6007,
            "name": "unauthorized",
            "msg": "unauthorized"
        }
    ],
    "types": [
        {
            "name": "global",
            "type": {
                "kind": "struct",
                "fields": [
                    {
                        "name": "admin",
                        "type": "pubkey"
                    },
                    {
                        "name": "usdcMint",
                        "type": "pubkey"
                    },
                    {
                        "name": "totalFund",
                        "type": "u64"
                    },
                    {
                        "name": "levelsEnabled",
                        "type": {
                            "array": [
                                "bool",
                                6
                            ]
                        }
                    },
                    {
                        "name": "totalPerLevel",
                        "type": {
                            "array": [
                                "u64",
                                6
                            ]
                        }
                    }
                ]
            }
        },
        {
            "name": "userFund",
            "type": {
                "kind": "struct",
                "fields": [
                    {
                        "name": "user",
                        "type": "pubkey"
                    },
                    {
                        "name": "totalFunded",
                        "type": "u64"
                    },
                    {
                        "name": "perLevel",
                        "type": {
                            "array": [
                                "u64",
                                6
                            ]
                        }
                    }
                ]
            }
        }
    ]
};
