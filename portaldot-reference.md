# Portaldot — Agent Reference

> Capability reference for agents building on **Portaldot**, distilled from the official docs:
> <https://portaldot-dev.readthedocs.io/en/latest/>
>
> Portaldot is a **Substrate-based** chain with a Polkadot-style runtime (FRAME pallets).
> Interact with it through the **`portaldot` Python SDK** (a `SubstrateInterface`-compatible client):
> query storage, compose/sign/submit extrinsics, call runtime APIs, and interface ink! contracts.

---

## 1. Chain facts

| Field | Value |
|---|---|
| Network | Portaldot (mainnet) |
| Token symbol | `POT` |
| Token decimals | **14** (1 POT = `10**14` planck) |
| SS58 prefix | **42** (generic Substrate) |
| WSS endpoint | `wss://mainnet.portaldot.io` |
| Account format | SS58 `5...` (sr25519/ed25519/ecdsa) |
| Node clients | Ubuntu / macOS binaries on GitHub; Windows via WSL |

High-level positioning (from Introduction): Layer0 "value OS" with dynamic heterogeneous sharding (256 shards), NPoS-style consensus ("LAO NPoS"), cross-chain + RWA tokenization, ZK/quantum-resistant privacy, ink! smart contracts. Treat the runtime as **standard FRAME** — the pallet/extrinsic/storage/event surface below is what actually drives integrations.

> ⚠️ Amounts are `u128` planck. Always convert: `value = pot * 10**14`. `Compact<u128>` args take a plain int.

---

## 2. Python SDK quickstart

Client object is referred to as `portaldot` (a `SubstrateInterface`).

### Connect / cleanup
```python
from substrateinterface import SubstrateInterface
portaldot = SubstrateInterface(url="wss://mainnet.portaldot.io")
# context-manager form auto-closes the socket:
with SubstrateInterface(url="wss://mainnet.portaldot.io") as portaldot:
    ...
```

### Keypairs & signing
```python
from substrateinterface import Keypair, KeypairType

mnemonic = Keypair.generate_mnemonic()
keypair  = Keypair.create_from_mnemonic(mnemonic)                 # sr25519 default
kp_ed    = Keypair.create_from_mnemonic(mnemonic, crypto_type=KeypairType.ED25519)
kp_uri   = Keypair.create_from_uri('//Alice')                     # dev account
kp_json  = Keypair.create_from_encrypted_json(json_data, passphrase, ss58_format=42)
verify   = Keypair(ss58_address="5...", crypto_type=KeypairType.SR25519)  # verify-only

addr = keypair.ss58_address
sig  = keypair.sign("Test123")
assert keypair.verify("Test123", sig)
```
Derivation: `mnemonic + '//hard/soft'`; BIP44/ECDSA: `f"{mnemonic}/m/44'/60'/0'/0/0"`.
Crypto types: `SR25519` (default), `ED25519`, `ECDSA`.

### Query storage
```python
# single item
acct = portaldot.query('System', 'Account', ['5Grw...'])
print(acct.value['nonce'], acct.value['data']['free'])

# at a specific block
portaldot.query(module='System', storage_function='Account',
                params=['5Grw...'], block_hash='0x...')

# batch
keys = [portaldot.create_storage_key("System","Account",["A1"]),
        portaldot.create_storage_key("System","Account",["A2"])]
portaldot.query_multi(keys)

# iterate a map
for acct_id, info in portaldot.query_map('System','Account', max_results=199):
    print(info.value['data']['free'])

# constants
const = portaldot.get_constant('Balances', 'ExistentialDeposit')
```

### Compose / sign / submit extrinsics
```python
call = portaldot.compose_call(
    call_module='Balances',
    call_function='transfer_keep_alive',
    call_params={'dest': '5E9o...', 'value': 1 * 10**14},   # 1 POT
)
extrinsic = portaldot.create_signed_extrinsic(call=call, keypair=keypair)
# mortal: create_signed_extrinsic(..., era={'period': 64})
try:
    receipt = portaldot.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    print("in block", receipt.block_hash)
    # receipt.is_success, receipt.triggered_events
except Exception as e:
    print("failed:", e)

# fee estimate before sending:
info = portaldot.get_payment_info(call=call, keypair=keypair)   # -> partialFee, weight
```
Multisig: `portaldot.generate_multisig_account(signatories, threshold)` + `portaldot.create_multisig_extrinsic(call, keypair, multisig_account, max_weight=...)`.
Offline: `generate_signature_payload(call, era, nonce)` → `keypair.sign(payload)` → `create_signed_extrinsic(call, keypair, era, nonce, signature)`.

### Runtime API calls
```python
portaldot.runtime_call("AccountNonceApi", "account_nonce", ["5Grw..."])
portaldot.get_metadata_runtime_call_functions()                       # discover
rc = portaldot.get_metadata_runtime_call_function("ContractsApi","call")
rc.get_param_info()
```

### ink! smart contracts
```python
from substrateinterface.contracts import ContractCode, ContractInstance

# deploy
code = ContractCode.create_from_contract_files(
    metadata_file='test_contract.json', wasm_file='test_contract.wasm', substrate=portaldot)
contract = code.deploy(keypair=keypair, endowment=0, gas_limit=1000000000000,
                       constructor="new", args={'init_value': True}, upload_code=True)

# attach to existing
contract = ContractInstance.create_from_address(
    contract_address=addr, metadata_file='test_contract.json', substrate=portaldot)

# read (dry-run, free)
res = contract.read(keypair, 'get'); print(res.contract_result_data)

# exec (state-changing) — predict gas via read, then exec
gas = contract.read(keypair, 'flip').gas_required
rcpt = contract.exec(keypair, 'flip', args={}, gas_limit=gas)
```

---

## 3. Pallet capability matrix (extrinsics = actions an agent can take)

`compose_call(call_module=<Pallet>, call_function=<snake_case>, call_params={...})`.
Names below are doc CamelCase; the SDK call_function is the snake_case form (`transferKeepAlive` → `transfer_keep_alive`).

### Balances — native POT transfers
| Call | Params |
|---|---|
| `transfer` | `source, dest: MultiAddress, value: Compact<u128>` |
| `transferKeepAlive` | `dest: MultiAddress, value: Compact<u128>` — won't reap origin |
| `transferAll` | `dest: MultiAddress, keep_alive: bool` |

### Assets — fungible asset classes (asset `id: u32`)
Create/lifecycle: `create(id, admin, min_balance)`, `forceCreate`, `startDestroy(id)`, `destroyAccounts(id)`, `destroyApprovals(id)`, `finishDestroy(id)`.
Supply: `mint(id, beneficiary, amount)`, `burn(id, who, amount)`.
Transfer: `transfer(id, target, amount)`, `transferKeepAlive(id, target, amount)`, `transferAll(id, dest, keep_alive)`, `forceTransfer(id, source, dest, amount)`.
Approvals: `approveTransfer(id, delegate, amount)`, `cancelApproval(id, delegate)`, `transferApproved(id, owner, destination, amount)`, `forceCancelApproval`.
Freeze/admin: `freeze(id, who)`/`thaw(id, who)`, `freezeAsset(id)`/`thawAsset(id)`, `block(id, who)`, `setTeam(id, issuer, admin, freezer)`, `transferOwnership(id, owner)`, `setMetadata(id, name, symbol, decimals)`/`clearMetadata`, `setMinBalance`, `touch(id)`/`touchOther`, `refund`/`refundOther`, `forceSetMetadata`/`forceClearMetadata`/`forceAssetStatus`.

### Staking — NPoS validation/nomination
Bond: `bond(value, payee)`, `bondExtra(max_additional)`, `unbond(value)`, `rebond(value)`, `withdrawUnbonded(num_slashing_spans)`.
Roles: `validate(prefs)`, `nominate(targets: Vec<MultiAddress>)`, `chill()`, `chillOther(stash)`, `kick(who)`.
Rewards: `payoutStakers(validator_stash, era)`, `payoutStakersByPage(validator_stash, era, page)`, `setPayee(payee)`, `updatePayee(controller)`, `setController()`.
Governance/force: `setValidatorCount`, `increaseValidatorCount`, `scaleValidatorCount`, `setStakingConfigs(...)`, `setMinCommission`, `forceApplyMinCommission`, `setInvulnerables`, `forceNewEra`/`forceNewEraAlways`/`forceNoEras`, `forceUnstake`, `manualSlash(validator_stash, era, slash_fraction)`, `reapStash`, `restoreLedger`, `migrateCurrency`, `deprecateControllerBatch`.
`RewardDestination` (payee): Staked / Stash / Controller / Account / None.

### Contracts — Wasm / ink!
`call(dest, value, gas_limit, storage_deposit_limit, data)`, `instantiate(value, gas_limit, storage_deposit_limit, code_hash, data, salt)`, `instantiateWithCode(value, gas_limit, storage_deposit_limit, code, data, salt)`, `uploadCode(code, storage_deposit_limit, determinism)`, `removeCode(code_hash)`, `setCode(dest, code_hash)`, `migrate(weight_limit)`. (`*OldWeight` variants are deprecated.)

### Utility — batching & origin tricks
`batch(calls)`, `batchAll(calls)` (atomic), `forceBatch(calls)` (continue on error), `asDerivative(index, call)`, `dispatchAs(as_origin, call)`, `dispatchAsFallible(as_origin, call)`, `ifElse(main, fallback)`, `withWeight(call, weight)`.

### Scheduler — on-chain time/block scheduling
`schedule(when, maybe_periodic, priority, call)`, `scheduleAfter(after, ...)`, `scheduleNamed(id, when, ...)`, `scheduleNamedAfter(id, after, ...)`, `cancel(when, index)`, `cancelNamed(id)`, `setRetry(task, retries, period)`, `setRetryNamed`, `cancelRetry`, `cancelRetryNamed`. `maybe_periodic = Option<(period, repetitions)>`.

### Multisig
`asMultiThreshold1(other_signatories, call)`, `asMulti(threshold, other_signatories, maybe_timepoint, call, max_weight)`, `approveAsMulti(threshold, other_signatories, maybe_timepoint, call_hash, max_weight)`, `cancelAsMulti(threshold, other_signatories, timepoint, call_hash)`, `pokeDeposit(...)`.

### Proxy — delegated calls
`addProxy(delegate, proxy_type, delay)`, `removeProxy(delegate, proxy_type, delay)`, `removeProxies()`, `proxy(real, force_proxy_type, call)`, `createPure(proxy_type, delay, index)`, `killPure(...)`, `announce(real, call_hash)`, `proxyAnnounced(...)`, `rejectAnnouncement`, `removeAnnouncement`, `pokeDeposit()`.

### Treasury
`spend(asset_kind, amount, beneficiary, valid_from)`, `spendLocal(amount, beneficiary)`, `payout(index)`, `checkStatus(index)`, `voidSpend(index)`, `removeApproval(proposal_id)`.

### Bounties
`proposeBounty(value, description)`, `approveBounty(bounty_id)`, `approveBountyWithCurator(bounty_id, curator, fee)`, `proposeCurator(bounty_id, curator, fee)`, `acceptCurator(bounty_id)`, `unassignCurator(bounty_id)`, `awardBounty(bounty_id, beneficiary)`, `claimBounty(bounty_id)`, `closeBounty(bounty_id)`, `extendBountyExpiry(bounty_id, remark)`.

### Vesting
`vest()`, `vestOther(target)`, `vestedTransfer(target, schedule)`, `forceVestedTransfer(source, target, schedule)`, `mergeSchedules(i1, i2)`, `forceRemoveVestingSchedule(target, schedule_index)`. `schedule = {locked, per_block, starting_block}`.

### Identity
`setIdentity(info)`, `clearIdentity()`, `killIdentity(target)`, `setSubs(subs)`, `addSub(sub, data)`, `removeSub(sub)`, `renameSub(sub, data)`, `quitSub()`, `requestJudgement(reg_index, max_fee)`, `cancelRequest(reg_index)`, `provideJudgement(...)`, `addRegistrar(account)`, `setFee`, `setFields`, `setAccountId`. Usernames: `addUsernameAuthority`, `removeUsernameAuthority`, `setUsernameFor`, `acceptUsername`, `setPrimaryUsername`, `unbindUsername`, `removeUsername`, `killUsername`, `removeExpiredApproval`.

### Indices
`claim(index)`, `transfer(new, index)`, `free(index)`, `forceTransfer(new, index, freeze)`, `freeze(index)`, `pokeDeposit(index)`.

### Lottery
`startLottery(price, length, delay, repeat)`, `stopRepeat()`, `setCalls(calls)`, `buyTicket(call)`.

### Session (validators)
`setKeys(keys, proof)`, `purgeKeys()`.

### System (mostly Root/governance)
`remark(remark)`, `remarkWithEvent(remark)`, `setCode(code)`, `setCodeWithoutChecks`, `authorizeUpgrade(code_hash)`, `applyAuthorizedUpgrade(code)`, `setStorage(items)`, `killStorage(keys)`, `killPrefix(prefix, subkeys)`, `setHeapPages(pages)`.

### Timestamp
`set(now)` — inherent, one per block.

Other consensus pallets present: **babe, grandpa, imOnline, authorship, offences, electionProviderMultiPhase, mmr, randomnessCollectiveFlip, transactionPayment, transactionStorage** (validator/runtime internals — rarely called directly).

---

## 4. Key storage reads (triggers / conditions)

### System
- `System.Account(AccountId)` → `{ nonce, consumers, providers, data:{ free, reserved, frozen, flags } }` — **the** balance + nonce read.
- `System.Number` → current block number · `System.BlockHash(u32)` → hash · `System.ParentHash` · `System.Events` (avoid on-chain) · `System.ExtrinsicCount`.

### Balances
- `Balances.TotalIssuance` / `InactiveIssuance` → `u128`.
- `Balances.Account(AccountId)`, `Locks`, `Holds`, `Freezes`, `Reserves` per account.

### Staking
- `Staking.ActiveEra` → `{index, start}` · `Staking.CurrentEra` → `u32`.
- `Staking.Ledger(controller)` → bonded ledger · `Staking.Bonded(stash)` → controller · `Staking.Payee(stash)` → reward dest.
- `Staking.Validators(stash)` → prefs · `Staking.Nominators(stash)` → nominations.
- `Staking.ErasValidatorReward(era)`, `ErasRewardPoints(era)`, `ErasStakersOverview(era, validator)`, `ErasStakersPaged(era, validator, page)`, `ClaimedRewards(era, validator)`.
- Config: `ValidatorCount`, `MinNominatorBond`, `MinValidatorBond`, `MinCommission`, `MaxValidatorsCount`, `MaxNominatorsCount`, `Invulnerables`, `ForceEra`.

### Assets
- `Assets.Asset(id)` → details (supply, owner, status) · `Assets.Account(id, who)` → holding · `Assets.Metadata(id)` → `{name, symbol, decimals}` · `Assets.Approvals(id, owner, delegate)` · `Assets.NextAssetId`.

---

## 5. Key events (watch for / parse from receipts)

Read from `receipt.triggered_events` after submission, or scan `System.Events` per block.

- **System**: `ExtrinsicSuccess(info)`, `ExtrinsicFailed(error, info)`, `NewAccount(who)`, `KilledAccount(who)`, `CodeUpdated`, `Remarked(who, hash)`.
- **Balances**: `Transfer(from, to, amount)`, `Deposit`, `Withdraw`, `Reserved`, `Unreserved`, `Slashed`, `Minted`, `Burned`, `Endowed`, `DustLost`, `Frozen`/`Thawed`, `Locked`/`Unlocked`, `BalanceSet`.
- **Staking**: `Bonded(who, amt)`, `Unbonded`, `Withdrawn`, `Rewarded(stash, dest, amt)`, `Slashed`, `SlashReported`, `EraPaid(era, validator_payout, remainder)`, `PayoutStarted`, `StakersElected`, `StakingElectionFailed`, `Chilled`, `Kicked`, `ValidatorPrefsSet`, `ForceEra`.
- **Assets**: `Created(id, creator, owner)`, `Issued(id, owner, amt)`, `Transferred(id, from, to, amt)`, `Burned`, `TransferredApproved`, `ApprovedTransfer`, `ApprovalCancelled`, `MetadataSet`/`MetadataCleared`, `OwnerChanged`, `TeamChanged`, `Frozen`/`Thawed`, `AssetFrozen`/`AssetThawed`, `Destroyed`, `Touched`, `Deposited`/`Withdrawn`.
- **Contracts**: `Instantiated(deployer, contract)`, `Called(caller, contract)`, `ContractEmitted(contract, data)` (custom ink! events), `CodeStored`/`CodeRemoved`/`ContractCodeUpdated`, `Terminated`, `DelegateCalled`, `StorageDepositTransferredAndHeld`/`...Released`.

---

## 6. JSON-RPC namespaces

`author`, `babe`, `chain`, `childstate`, `contracts`, `grandpa`, `mmr`, `offchain`, `payment`, `rpc`, `state`, `syncstate`, `system`.
Common: `chain_getBlock`/`getFinalizedHead`/`subscribeNewHeads`, `state_getStorage`/`getRuntimeVersion`/`call`/`subscribeStorage`, `author_submitExtrinsic`/`submitAndWatchExtrinsic`, `payment_queryInfo`/`queryFeeDetails`, `contracts_call`/`instantiate`, `system_chain`/`health`/`properties`. The SDK wraps these — prefer `query`/`submit_extrinsic`/`runtime_call` over raw RPC.

---

## 7. Quick recipes

```python
# Read free balance (POT)
free_planck = portaldot.query('System','Account',[addr]).value['data']['free']
free_pot = free_planck / 10**14

# Atomic multi-transfer
calls = [portaldot.compose_call('Balances','transfer_keep_alive',{'dest':d,'value':v}) for d,v in pays]
batch = portaldot.compose_call('Utility','batch_all',{'calls':calls})
portaldot.submit_extrinsic(portaldot.create_signed_extrinsic(batch, keypair), wait_for_inclusion=True)

# Claim + re-bond staking rewards
payout = portaldot.compose_call('Staking','payout_stakers',{'validator_stash':v,'era':era})
bond   = portaldot.compose_call('Staking','bond_extra',{'max_additional': amount})
seq    = portaldot.compose_call('Utility','batch',{'calls':[payout, bond]})

# Schedule a call N blocks out
inner = portaldot.compose_call('Balances','transfer_keep_alive',{'dest':d,'value':v})
sched = portaldot.compose_call('Scheduler','schedule_after',
        {'after':100,'maybe_periodic':None,'priority':0,'call':inner})
```

---

## 8. Source pages
- Introduction: `/Introduction.html` · Chain info: `/chain-info.html`
- SDK: `/python-sdk/usage/{usage,keypair-creation-and-signing,query-storage,extrinsics,call-runtime-apis,ink-contract-interfacing,cleanup-and-context-manager}.html`
- Module interface indexes: `/module-interface/{extrinsics,storage,events,rpc,errors,constants}/index.html` (per-pallet pages under each)

_Base: <https://portaldot-dev.readthedocs.io/en/latest/>. Generated for agent use; verify args against live runtime metadata (`portaldot.get_metadata_call_function(pallet, call)`) before submitting._
