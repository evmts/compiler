object "YulEcho" {
  code {
    datacopy(0, dataoffset("runtime"), datasize("runtime"))
    return(0, datasize("runtime"))
  }

  object "runtime" {
    code {
      mstore(0x00, 0x20)
      mstore(0x20, 0x00)
      return(0x1c, 0x24)
    }
  }
}
