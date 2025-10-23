// SPDX-License-Identifier: BSD-3-Clause
pragma solidity ^0.8.30;

/// @notice Minimal subset of Compound's CErc20Delegate used for invariant shadowing tests.
contract CErc20DelegateMock {
	uint internal totalCash;
	uint internal totalBorrows;
	uint internal totalReserves;
	uint internal totalSupply_;
	uint internal reserveFactorMantissa;
	uint internal accrualBlockNumber;
	uint internal borrowIndex;

	event Borrow(address indexed borrower, uint borrowAmount);

	function setTotals(
		uint cash,
		uint borrows,
		uint reserves,
		uint supply
	) external {
		totalCash = cash;
		totalBorrows = borrows;
		totalReserves = reserves;
		totalSupply_ = supply;
	}

	function setAccounting(uint newReserveFactorMantissa, uint newAccrualBlockNumber, uint newBorrowIndex) external {
		reserveFactorMantissa = newReserveFactorMantissa;
		accrualBlockNumber = newAccrualBlockNumber;
		borrowIndex = newBorrowIndex;
	}

	function getCash() public view returns (uint) {
		return totalCash;
	}

	function totalSupply() public view returns (uint) {
		return totalSupply_;
	}

	function totalBorrowsCurrent() external view returns (uint) {
		return totalBorrows;
	}

	function totalReservesMantissa() external view returns (uint) {
		return totalReserves;
	}

	function exchangeRateStoredInternal() internal view returns (uint) {
		require(totalSupply_ > 0, 'division by zero');
		// Intentional bug: attempting to scale reserves by the reserve factor but 
		// truncating the result to zero for small values.
		uint adjustedReserves = (totalReserves * reserveFactorMantissa) / 1e18;
		uint numerator = (totalCash + totalBorrows - adjustedReserves) * 1e18;
		return numerator / totalSupply_;
	}

	function exchangeRateStored() external view returns (uint) {
		return exchangeRateStoredInternal();
	}

	function borrow(uint borrowAmount) external {
		uint reserveIncrease = (borrowAmount * reserveFactorMantissa) / 1e18;
		totalBorrows += borrowAmount;
		totalCash -= borrowAmount;
		totalReserves += reserveIncrease;
		emit Borrow(msg.sender, borrowAmount);
	}
}
