// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.30;
pragma abicoder v2;

/// @title BitMath
/// @dev Open-source library from Uniswap v3-core (GPL-2.0-or-later)
library BitMath {
	function mostSignificantBit(uint256 x) internal pure returns (uint8 r) {
		require(x > 0);
		if (x >= 0x100000000000000000000000000000000) {
			x >>= 128;
			r += 128;
		}
		if (x >= 0x10000000000000000) {
			x >>= 64;
			r += 64;
		}
		if (x >= 0x100000000) {
			x >>= 32;
			r += 32;
		}
		if (x >= 0x10000) {
			x >>= 16;
			r += 16;
		}
		if (x >= 0x100) {
			x >>= 8;
			r += 8;
		}
		if (x >= 0x10) {
			x >>= 4;
			r += 4;
		}
		if (x >= 0x4) {
			x >>= 2;
			r += 2;
		}
		if (x >= 0x2) r += 1;
	}

	function leastSignificantBit(uint256 x) internal pure returns (uint8 r) {
		require(x > 0);
		r = 255;
		if (x & type(uint128).max > 0) {
			r -= 128;
		} else {
			x >>= 128;
		}
		if (x & type(uint64).max > 0) {
			r -= 64;
		} else {
			x >>= 64;
		}
		if (x & type(uint32).max > 0) {
			r -= 32;
		} else {
			x >>= 32;
		}
		if (x & type(uint16).max > 0) {
			r -= 16;
		} else {
			x >>= 16;
		}
		if (x & type(uint8).max > 0) {
			r -= 8;
		} else {
			x >>= 8;
		}
		if (x & 0xf > 0) {
			r -= 4;
		} else {
			x >>= 4;
		}
		if (x & 0x3 > 0) {
			r -= 2;
		} else {
			x >>= 2;
		}
		if (x & 0x1 > 0) r -= 1;
	}
}

/// @title Packed tick initialized state library
/// @dev Adapted from Uniswap v3-core TickBitmap.sol (BUSL-1.1)
library TickBitmap {
	function position(int24 tick) private pure returns (int16 wordPos, uint8 bitPos) {
		wordPos = int16(tick >> 8);
		int256 mod = tick % 256;
		if (mod < 0) {
			mod += 256;
		}
		bitPos = uint8(uint256(mod));
	}

	function flipTick(
		mapping(int16 => uint256) storage self,
		int24 tick,
		int24 tickSpacing
	) internal {
		require(tick % tickSpacing == 0, 'Tick not aligned');
		(int16 wordPos, uint8 bitPos) = position(tick / tickSpacing);
		uint256 mask = 1 << bitPos;
		self[wordPos] ^= mask;
	}

	function getWord(mapping(int16 => uint256) storage self, int16 wordPos) internal view returns (uint256) {
		return self[wordPos];
	}
}

/// @notice Minimal Uniswap v3 pool surface used for integration testing.
contract UniswapV3PoolMock {
	using TickBitmap for mapping(int16 => uint256);

	mapping(int16 => uint256) private _tickBitmap;

	function flipTick(int24 tick, int24 tickSpacing) external {
		_tickBitmap.flipTick(tick, tickSpacing);
	}

	function setTickWord(int16 wordIndex, uint256 word) external {
		_tickBitmap[wordIndex] = word;
	}

	function tickBitmap(int16 wordIndex) external view returns (uint256) {
		return _tickBitmap.getWord(wordIndex);
	}
}
