# Documentation of Un-Mix program

## Specification

The program is an advanced fund me.

| Level    | Total fund (USDC) | Per account max fund (USDC) |
| -------- | ----------------- | --------------------------- |
| 1.       | 5 400 000         | 500                         |
| 2.       | 88 000 000        | 999                         |
| 3.       | 173 000 000       | 5000                        |
| 4.       | 116 250 000       | Unlimited                   |
| 5.       | 45 800 000        | Unlimited                   |
| 6.       | 37 200 000        | Unlimited                   |

In admin:

Initialize - Initialize and enable the contract

EnableLevel - Enables the nth level via true or false parameter

Withdraw - Withdraws the funds

For users:

Fund - It will fund level via number parameter

## Core flow

Stats:

- Total fund
- Total fund for each level
- Total fund per User
- Fund per User per level
- Level is Enabled

Function for each level

## Global rules

Minimum 1 USDC in fund
