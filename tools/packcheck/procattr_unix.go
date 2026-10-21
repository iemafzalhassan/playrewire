//go:build !windows

package main

import (
	"os/exec"
	"syscall"
)

// setProcessGroup puts the toolchain in its own process group so a timeout
// kills the compiled program too, not just the go command.
func setProcessGroup(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error {
		return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
	}
}
