export type ContractTag =
  | 'basics' | 'math' | 'tokens' | 'payable' | 'access'
  | 'cross-contract' | 'oop' | 'strings' | 'arrays';

export interface ContractEntry {
  name: string;
  tags: ContractTag[];
  description: string;
  constructor: string;
  messages: string[];
  testSteps: number;
  solFile: string;
  testFile: string;
}

const REPO = 'https://github.com/freedanjeremiah/inkide/blob/main';

export const CONTRACTS: ContractEntry[] = [
  { name: 'Counter', tags: ['basics'], description: 'Stateful increment with constructor arg and view getter.', constructor: 'constructor(uint256 initial)', messages: ['inc()', 'incBy(uint256)', 'get()'], testSteps: 5, solFile: `${REPO}/contracts/Counter.sol`, testFile: `${REPO}/tests/Counter.json` },
  { name: 'Flipper', tags: ['basics'], description: 'Boolean toggle — simplest possible stateful contract.', constructor: 'constructor(bool init)', messages: ['flip()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Flipper.sol`, testFile: `${REPO}/tests/Flipper.json` },
  { name: 'SimpleStorage', tags: ['basics'], description: 'Single uint store and retrieve.', constructor: 'constructor()', messages: ['set(uint256)', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/SimpleStorage.sol`, testFile: `${REPO}/tests/SimpleStorage.json` },
  { name: 'Pub', tags: ['basics'], description: 'Public variable auto-getter pattern.', constructor: 'constructor(uint256)', messages: ['value()'], testSteps: 2, solFile: `${REPO}/contracts/Pub.sol`, testFile: `${REPO}/tests/Pub.json` },
  { name: 'Inc', tags: ['basics'], description: 'Increment-only counter.', constructor: 'constructor()', messages: ['inc()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Inc.sol`, testFile: `${REPO}/tests/Inc.json` },
  { name: 'Sum', tags: ['math'], description: 'Accumulator with running total.', constructor: 'constructor()', messages: ['add(uint256)', 'total()'], testSteps: 4, solFile: `${REPO}/contracts/Sum.sol`, testFile: `${REPO}/tests/Sum.json` },
  { name: 'MinMax', tags: ['math'], description: 'Tracks minimum and maximum of submitted values.', constructor: 'constructor()', messages: ['submit(uint256)', 'min()', 'max()'], testSteps: 5, solFile: `${REPO}/contracts/MinMax.sol`, testFile: `${REPO}/tests/MinMax.json` },
  { name: 'Bits', tags: ['math'], description: 'Bitwise operators: &, |, ^, ~, <<, >>.', constructor: 'constructor()', messages: ['and(uint256,uint256)', 'or(uint256,uint256)', 'xor(uint256,uint256)'], testSteps: 6, solFile: `${REPO}/contracts/Bits.sol`, testFile: `${REPO}/tests/Bits.json` },
  { name: 'Signed', tags: ['math'], description: 'Signed i128 arithmetic with overflow protection.', constructor: 'constructor(int256)', messages: ['add(int256)', 'get()'], testSteps: 4, solFile: `${REPO}/contracts/Signed.sol`, testFile: `${REPO}/tests/Signed.json` },
  { name: 'NarrowMath', tags: ['math'], description: 'uint16 overflow reverts at declared width.', constructor: 'constructor()', messages: ['add(uint16,uint16)'], testSteps: 4, solFile: `${REPO}/contracts/NarrowMath.sol`, testFile: `${REPO}/tests/NarrowMath.json` },
  { name: 'Narrow16', tags: ['math'], description: 'Width-checked arithmetic on uint16 variables.', constructor: 'constructor(uint16)', messages: ['inc()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Narrow16.sol`, testFile: `${REPO}/tests/Narrow16.json` },
  { name: 'Unchecked', tags: ['math'], description: 'unchecked{} wraps instead of reverts.', constructor: 'constructor()', messages: ['wrap(uint256,uint256)'], testSteps: 3, solFile: `${REPO}/contracts/Unchecked.sol`, testFile: `${REPO}/tests/Unchecked.json` },
  { name: 'Cast', tags: ['math'], description: 'Narrowing cast: uint8(256) == 0.', constructor: 'constructor()', messages: ['cast8(uint256)'], testSteps: 3, solFile: `${REPO}/contracts/Cast.sol`, testFile: `${REPO}/tests/Cast.json` },
  { name: 'ERC20', tags: ['tokens'], description: 'Fungible token: transfer, approve, allowance, events.', constructor: 'constructor(uint256 initialSupply)', messages: ['transfer(address,uint256)', 'transferFrom(address,address,uint256)', 'approve(address,uint256)', 'balanceOf(address)', 'allowance(address,address)'], testSteps: 8, solFile: `${REPO}/contracts/ERC20.sol`, testFile: `${REPO}/tests/ERC20.json` },
  { name: 'ERC721', tags: ['tokens'], description: 'Non-fungible token: mint, transfer, ownership.', constructor: 'constructor()', messages: ['mint(address,uint256)', 'transfer(address,uint256)', 'ownerOf(uint256)'], testSteps: 6, solFile: `${REPO}/contracts/ERC721.sol`, testFile: `${REPO}/tests/ERC721.json` },
  { name: 'Ownable', tags: ['access'], description: 'onlyOwner modifier and ownership transfer.', constructor: 'constructor()', messages: ['transferOwnership(address)', 'owner()'], testSteps: 4, solFile: `${REPO}/contracts/Ownable.sol`, testFile: `${REPO}/tests/Ownable.json` },
  { name: 'Bank', tags: ['payable'], description: 'msg.value deposit/withdraw with balance tracking.', constructor: 'constructor()', messages: ['deposit()', 'withdraw(uint256)', 'balanceOf(address)'], testSteps: 5, solFile: `${REPO}/contracts/Bank.sol`, testFile: `${REPO}/tests/Bank.json` },
  { name: 'Escrow', tags: ['payable'], description: 'Conditional release of held funds between parties.', constructor: 'constructor(address beneficiary)', messages: ['deposit()', 'release()'], testSteps: 5, solFile: `${REPO}/contracts/Escrow.sol`, testFile: `${REPO}/tests/Escrow.json` },
  { name: 'Auction', tags: ['payable'], description: 'Timed bidding with highest-bidder tracking.', constructor: 'constructor(uint256 duration)', messages: ['bid()', 'end()', 'highestBidder()', 'highestBid()'], testSteps: 7, solFile: `${REPO}/contracts/Auction.sol`, testFile: `${REPO}/tests/Auction.json` },
  { name: 'Voting', tags: ['access'], description: 'Proposal creation, vote recording, result query.', constructor: 'constructor()', messages: ['addProposal(string)', 'vote(uint256)', 'winner()'], testSteps: 6, solFile: `${REPO}/contracts/Voting.sol`, testFile: `${REPO}/tests/Voting.json` },
  { name: 'Greeter', tags: ['strings'], description: 'string storage and retrieval.', constructor: 'constructor(string memory)', messages: ['greet()', 'setGreeting(string)'], testSteps: 3, solFile: `${REPO}/contracts/Greeter.sol`, testFile: `${REPO}/tests/Greeter.json` },
  { name: 'IntList', tags: ['arrays'], description: 'Dynamic uint[] array with push, length, index access.', constructor: 'constructor()', messages: ['push(uint256)', 'get(uint256)', 'length()'], testSteps: 5, solFile: `${REPO}/contracts/IntList.sol`, testFile: `${REPO}/tests/IntList.json` },
  { name: 'Structs', tags: ['oop'], description: 'Struct locals and field access.', constructor: 'constructor()', messages: ['store(uint256,address)', 'retrieve(uint256)'], testSteps: 4, solFile: `${REPO}/contracts/Structs.sol`, testFile: `${REPO}/tests/Structs.json` },
  { name: 'Enum', tags: ['oop'], description: 'Enum state machine with transitions.', constructor: 'constructor()', messages: ['advance()', 'state()'], testSteps: 4, solFile: `${REPO}/contracts/Enum.sol`, testFile: `${REPO}/tests/Enum.json` },
  { name: 'Inherit', tags: ['oop'], description: 'Inheritance flattening (is Base) with overridden methods.', constructor: 'constructor()', messages: ['value()', 'double()'], testSteps: 3, solFile: `${REPO}/contracts/Inherit.sol`, testFile: `${REPO}/tests/Inherit.json` },
  { name: 'Caller', tags: ['cross-contract'], description: 'Calls into Target via IFoo(addr).bar(args) cross-contract call.', constructor: 'constructor(address target)', messages: ['callTarget(uint256)', 'result()'], testSteps: 5, solFile: `${REPO}/contracts/Caller.sol`, testFile: `${REPO}/tests/Caller.json` },
  { name: 'Target', tags: ['cross-contract'], description: 'Deployed as dependency, referenced via @label in test specs.', constructor: 'constructor()', messages: ['set(uint256)', 'get()'], testSteps: 2, solFile: `${REPO}/contracts/Target.sol`, testFile: `${REPO}/tests/Target.json` },
  { name: 'Overload', tags: ['oop'], description: 'Function overloading with distinct keccak4 selectors.', constructor: 'constructor()', messages: ['add(uint256)', 'add(uint256,uint256)'], testSteps: 4, solFile: `${REPO}/contracts/Overload.sol`, testFile: `${REPO}/tests/Overload.json` },
  { name: 'IdStore', tags: ['strings'], description: 'bytes32 storage keyed by address.', constructor: 'constructor()', messages: ['store(bytes32)', 'load(address)'], testSteps: 3, solFile: `${REPO}/contracts/IdStore.sol`, testFile: `${REPO}/tests/IdStore.json` },
  { name: 'Timed', tags: ['access'], description: 'block.timestamp gating — only callable within time window.', constructor: 'constructor(uint256 openAt, uint256 closeAt)', messages: ['action()', 'isOpen()'], testSteps: 4, solFile: `${REPO}/contracts/Timed.sol`, testFile: `${REPO}/tests/Timed.json` },
];

export const ALL_TAGS: ContractTag[] = ['basics', 'math', 'tokens', 'payable', 'access', 'cross-contract', 'oop', 'strings', 'arrays'];
