package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
)

// Wrapped-SOL mint on Solana — onchainos uses this address for native SOL price queries.
const solAddress = "So11111111111111111111111111111111111111112"

type onchainosResp struct {
	Ok    bool            `json:"ok"`
	Error string          `json:"error,omitempty"`
	Data  json.RawMessage `json:"data,omitempty"`
}

func main() {
	if len(os.Args) > 1 {
		switch os.Args[1] {
		case "-h", "--help":
			fmt.Println("Usage: sol-price-checker")
			fmt.Println("  Prints SOL price JSON from onchainos token price-info.")
			return
		case "-v", "--version":
			fmt.Println("sol-price-checker 1.0.0")
			return
		}
	}

	out, err := exec.Command("onchainos", "token", "price-info",
		"--address", solAddress, "--chain", "solana").Output()
	if err != nil {
		fmt.Fprintf(os.Stderr, "ERROR: failed to invoke onchainos: %v\n", err)
		if ee, ok := err.(*exec.ExitError); ok && len(ee.Stderr) > 0 {
			os.Stderr.Write(ee.Stderr)
		}
		os.Exit(1)
	}

	var r onchainosResp
	if err := json.Unmarshal(out, &r); err != nil {
		fmt.Fprintf(os.Stderr, "ERROR: invalid JSON from onchainos: %v\n", err)
		os.Exit(1)
	}
	if !r.Ok {
		fmt.Fprintf(os.Stderr, "onchainos error: %s\n", r.Error)
		os.Exit(1)
	}

	fmt.Println(string(r.Data))
}
