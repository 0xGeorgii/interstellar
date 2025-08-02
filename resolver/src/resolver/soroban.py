from stellar_sdk import (
    Server, 
    Keypair, 
    Network, 
    TransactionBuilder,
    Account,
    TransactionEnvelope
)
from stellar_sdk import SorobanServer
from stellar_sdk.exceptions import Ed25519SecretSeedInvalidError
import asyncio
import json
from typing import Dict, Any, Optional

class SorobanAuthExecutor:
    def __init__(self, server_secret_key: str, network: str = "TESTNET"):
        self.server_keypair = Keypair.from_secret(server_secret_key)
        self.network = Network.TESTNET_NETWORK_PASSPHRASE if network == "TESTNET" else Network.PUBLIC_NETWORK_PASSPHRASE
        
        if network == "TESTNET":
            self.horizon_server = Server("https://horizon-testnet.stellar.org")
            self.soroban_server = SorobanServer("https://soroban-testnet.stellar.org")
        else:
            self.horizon_server = Server("https://horizon.stellar.org")
            self.soroban_server = SorobanServer("https://soroban-mainnet.stellar.org")

    async def execute_with_authorization(
        self, 
        prepared_transaction_xdr: str,
        user_public_key: str
    ) -> Dict[str, Any]:
        """
        Execute a pre-authorized transaction from the client
        """
        try:
            # Parse the prepared transaction
            prepared_transaction = TransactionEnvelope.from_xdr(
                prepared_transaction_xdr, 
                self.network
            )
            
            # Get server account for fee payment
            server_account = self.horizon_server.load_account(
                self.server_keypair.public_key
            )
            
            # Clone the transaction with server as fee source
            final_transaction = self._rebuild_transaction_with_server_fee(
                prepared_transaction,
                server_account
            )
            
            # Sign with server keypair for fee payment
            final_transaction.sign(self.server_keypair)
            
            # Submit the transaction
            response = self.soroban_server.send_transaction(final_transaction)
            
            # Handle the response
            if response.status == "PENDING":
                # Wait for confirmation
                confirmed = await self._wait_for_confirmation(response.hash)
                return {
                    "status": "success",
                    "hash": response.hash,
                    "result": confirmed
                }
            elif response.status == "ERROR":
                return {
                    "status": "error", 
                    "error": response.error_result_xdr
                }
            else:
                return {
                    "status": response.status,
                    "hash": response.hash
                }
                
        except Exception as e:
            return {
                "status": "error",
                "error": str(e)
            }

    def _rebuild_transaction_with_server_fee(
        self, 
        original_transaction: TransactionEnvelope,
        server_account: Account
    ) -> TransactionEnvelope:
        """
        Rebuild the transaction using the server account for fee payment
        but preserving the original authorization signatures
        """
        # Create new transaction builder with server as source
        builder = TransactionBuilder(
            source_account=server_account,
            network_passphrase=self.network,
            base_fee=original_transaction.transaction.fee
        )
        
        # Add all operations from the original transaction
        for operation in original_transaction.transaction.operations:
            builder.add_operation(operation)
        
        # Set the same timeout
        builder.set_timeout(original_transaction.transaction.time_bounds.max_time - 
                          original_transaction.transaction.time_bounds.min_time
                          if original_transaction.transaction.time_bounds else 30)
        
        return builder.build()

    async def _wait_for_confirmation(
        self, 
        transaction_hash: str, 
        timeout: int = 30
    ) -> Optional[Dict]:
        """
        Wait for transaction confirmation
        """
        import time
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            try:
                result = self.soroban_server.get_transaction(transaction_hash)
                if result.status != "NOT_FOUND":
                    return result.__dict__
            except Exception:
                pass
            
            await asyncio.sleep(2)
        
        return None

    async def execute_contract_call_with_auth(
        self,
        user_public_key: str,
        contract_id: str,
        method: str,
        args: list,
        user_auth_signatures: list
    ) -> Dict[str, Any]:
        """
        Alternative method: Build and execute contract call with provided auth signatures
        """
        try:
            from stellar_sdk.operation import InvokeContract
            from stellar_sdk import scval
            
            # Get server account
            server_account = self.horizon_server.load_account(
                self.server_keypair.public_key
            )
            
            # Convert arguments to ScVal
            sc_args = []
            for arg in args:
                if isinstance(arg, str) and arg.startswith('G'):  # Address
                    sc_args.append(scval.to_address(arg))
                elif isinstance(arg, int):
                    sc_args.append(scval.to_int64(arg))
                elif isinstance(arg, str):
                    sc_args.append(scval.to_symbol(arg))
                else:
                    sc_args.append(scval.to_bytes(arg))
            
            # Build transaction
            transaction = (
                TransactionBuilder(
                    source_account=server_account,
                    network_passphrase=self.network,
                    base_fee=100000
                )
                .add_operation(
                    InvokeContract(
                        contract_address=contract_id,
                        function_name=method,
                        parameters=sc_args,
                        auth=user_auth_signatures  # Pre-signed auth from client
                    )
                )
                .set_timeout(30)
                .build()
            )
            
            # Simulate first to get accurate fee and resource requirements
            simulate_response = self.soroban_server.simulate_transaction(transaction)
            
            if hasattr(simulate_response, 'error'):
                return {
                    "status": "error",
                    "error": f"Simulation failed: {simulate_response.error}"
                }
            
            # Prepare transaction with simulation results
            prepared_transaction = self.soroban_server.prepare_transaction(
                transaction, simulate_response
            )
            
            # Sign with server keypair
            prepared_transaction.sign(self.server_keypair)
            
            # Submit
            response = self.soroban_server.send_transaction(prepared_transaction)
            
            if response.status == "PENDING":
                confirmed = await self._wait_for_confirmation(response.hash)
                return {
                    "status": "success",
                    "hash": response.hash,
                    "result": confirmed
                }
            else:
                return {
                    "status": response.status,
                    "hash": response.hash if hasattr(response, 'hash') else None,
                    "error": response.error_result_xdr if hasattr(response, 'error_result_xdr') else None
                }
                
        except Exception as e:
            return {
                "status": "error",
                "error": str(e)
            }
