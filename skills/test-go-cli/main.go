package main

import (
	"fmt"
	"os"
	"os/exec"
)

func main() {
	if len(os.Args) > 1 && os.Args[1] == "--query" && len(os.Args) > 2 && os.Args[2] == "eth-price" {
		fmt.Println("Querying ETH price via onchainos...")
		cmd := exec.Command("onchainos", "token", "price-info", "--address", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "--chain", "ethereum")
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		cmd.Run()
	} else if len(os.Args) > 1 && os.Args[1] == "--help" {
		fmt.Println("test-go-cli v1.0.0")
		fmt.Println("Usage: test-go-cli --query eth-price")
		fmt.Println("Queries ETH price via onchainos token price ETH")
	} else {
		fmt.Println("test-go-cli v1.0.0 - E2E test Go CLI")
		fmt.Println("Run with --help for usage")
	}
}
