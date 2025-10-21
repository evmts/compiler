counter: public(uint256)

@external
def increment():
  self.counter += 1

@external
def value() -> uint256:
  return self.counter
