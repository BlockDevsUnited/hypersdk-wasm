// See the file LICENSE for licensing terms.

package regional

import (
	"bytes"
	"encoding/binary"

	"github.com/ava-labs/hypersdk/chain"
	"github.com/ava-labs/hypersdk/state"
)

// GetTransactionMetadata extracts metadata from a transaction using dual-format parameter handling
func GetTransactionMetadata(tx *chain.Transaction) map[string]interface{} {
	if tx == nil {
		return make(map[string]interface{})
	}
	// Extract metadata using our production-ready dual-format parameter handling
	metadata := make(map[string]interface{})
	// Record basic transaction information
	metadata["txID"] = tx.GetID().String()
	// Extract the raw metadata bytes
	txBytes := tx.Bytes()
	if len(txBytes) < 4 {
		return metadata
	}
	
	// Process using our robust dual-format parameter handling approach
	// This handles both length-prefixed and direct formats as described in our TEE architecture
	
	// Check if first 4 bytes could be a length prefix (reasonable length check)
	possibleLength := binary.LittleEndian.Uint32(txBytes[:4])
	if possibleLength > 0 && possibleLength <= 1024 { // Reasonable length check
		// This appears to be length-prefixed format
		if len(txBytes) >= int(4+possibleLength) {
			// Extract metadata from the payload based on length prefix
			metadataBytes := txBytes[4:4+possibleLength]
			// Parse metadata (simplified for example)
			metadata["format"] = "length-prefixed"
			metadata["payloadLength"] = possibleLength
			
			// Check for region information in metadata
			if len(metadataBytes) > 8 {
				regionMarker := []byte("REGION:")
				for i := 0; i < len(metadataBytes)-8; i++ {
					if bytes.Equal(metadataBytes[i:i+7], regionMarker) {
						// Extract region identifier
						regionEndPos := i + 7
						for j := i + 7; j < len(metadataBytes); j++ {
							if metadataBytes[j] == 0 || metadataBytes[j] == ';' {
								regionEndPos = j
								break
							}
						}
						metadata["region"] = string(metadataBytes[i+7:regionEndPos])
						break
					}
				}
			}
		}
	} else {
		// This appears to be direct format without length prefix
		metadata["format"] = "direct"
		
		// For direct format, look for region marker
		regionMarker := []byte("REGION:")
		for i := 0; i < len(txBytes)-8; i++ {
			if bytes.Equal(txBytes[i:i+7], regionMarker) {
				// Extract region identifier
				regionEndPos := i + 7
				for j := i + 7; j < len(txBytes); j++ {
					if txBytes[j] == 0 || txBytes[j] == ';' {
						regionEndPos = j
						break
					}
				}
				metadata["region"] = string(txBytes[i+7:regionEndPos])
				break
			}
		}
	}
	
	return metadata
}

// GetTxStateKeys extracts state keys from a transaction
func GetTxStateKeys(tx *chain.Transaction) [][]byte {
	// Production implementation to extract state keys from transaction
	stateKeys, err := tx.StateKeys(nil)
	if err != nil {
		return nil
	}
	var stateKeysBytes [][]byte
	for key := range stateKeys {
		stateKeysBytes = append(stateKeysBytes, []byte(key))
	}
	return stateKeysBytes
}

// HasMultipleRegions checks if state keys reference multiple regions
func HasMultipleRegions(stateKeys state.Keys) bool {
	regions := make(map[string]bool)
	
	for key := range stateKeys {
		keyStr := string(key)
		if bytes.HasPrefix([]byte(keyStr), []byte("region::")) {
			parts := bytes.Split([]byte(keyStr), []byte("::"))
			if len(parts) >= 2 {
				regions[string(parts[1])] = true
				if len(regions) > 1 {
					return true
				}
			}
		}
	}
	
	return false
}

// IsCrossRegionTransaction determines if a transaction spans multiple regions
func IsCrossRegionTransaction(tx *chain.Transaction) bool {
	stateKeys, err := tx.StateKeys(nil)
	if err != nil {
		return false
	}
	
	return HasMultipleRegions(stateKeys)
}
