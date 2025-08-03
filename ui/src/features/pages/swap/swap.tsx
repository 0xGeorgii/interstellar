import React, { useState, useEffect, useCallback } from 'react';
import {
    Box, Grid, Paper, styled,
    Typography, Select, MenuItem, InputLabel,
    TextField, Button, Stack, Chip, InputAdornment,
    Accordion,
    AccordionSummary,
    AccordionDetails,
    ListItemIcon,
    ListItemText,
} from '@mui/material';
import { LocalizationProvider } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import { Horizon } from '@stellar/stellar-sdk';
import {
    StellarWalletsKit, WalletNetwork,
    FreighterModule, FREIGHTER_ID
} from '@creit.tech/stellar-wallets-kit';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { postOrder, postSecret } from '../../../api/interstellar/interstellar-api';
import { OrderData, Order, Signature } from '../../../api/interstellar/models/order';
import { ethers, parseEther, randomBytes } from 'ethers';
// Extend the Window interface to include the ethereum property
declare global {
    interface Window {
        ethereum?: any;
    }
}

const testnetServer = new Horizon.Server('https://horizon-testnet.stellar.org');
const kit = new StellarWalletsKit({
    network: WalletNetwork.TESTNET,
    selectedWalletId: FREIGHTER_ID,
    modules: [new FreighterModule()],
});

const Item = styled(Paper)(({ theme }) => ({
    backgroundColor: '#fff',
    padding: theme.spacing(2),
    textAlign: 'center',
    color: theme.palette.text.secondary,
}));

export const Swap: React.FC = () => {
    const [tokensPair, setTokensPair] = useState('');
    const [freighterConnected, setFreighterConnected] = useState(false);
    const [metamaskConnected, setMetamaskConnected] = useState(false);
    const [stellarAddress, setStellarAddress] = useState<string | null>(null);
    const [evmAddress, setEvmAddress] = useState<string | null>(null);
    const [xlmBalance, setXlmBalance] = useState<number | null>(null);
    const [ethBalance, setEthBalance] = useState<number | null>(null);
    const [fromAmount, setFromAmount] = useState<string>('');
    const [advancedOpen, setAdvancedOpen] = useState(false);
    const [resolverAddress, setResolverAddress] = useState<string>('');

    const updateEvmAccount = useCallback(async (address: string | null) => {
        if (!address) {
            setMetamaskConnected(false);
            setEvmAddress(null);
            setEthBalance(null);
            return;
        }
        setEvmAddress(address);
        setMetamaskConnected(true);
        const provider = new ethers.BrowserProvider(window.ethereum);
        const bal = await provider.getBalance(address);
        setEthBalance(parseFloat(ethers.formatEther(bal)));
    }, []);

    // Effect to handle account changes from the MetaMask extension
    useEffect(() => {
        if (!window.ethereum) return;

        const handleAccountsChanged = (accounts: string[]) => {
            if (accounts.length > 0) {
                // An account is connected or has been changed.
                updateEvmAccount(accounts[0]);
            } else {
                // User has disconnected all accounts in MetaMask.
                updateEvmAccount(null);
            }
        };

        window.ethereum.on('accountsChanged', handleAccountsChanged);

        // Cleanup function to remove the event listener when the component unmounts.
        return () => {
            window.ethereum.removeListener('accountsChanged', handleAccountsChanged);
        };
    }, [updateEvmAccount]);

    // Updated MetaMask connector using the helper function
    const connectMetamask = async () => {
        if (!window.ethereum) {
            alert('MetaMask not installed');
            return;
        }
        try {
            const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
            if (accounts.length > 0) {
                await updateEvmAccount(accounts[0]);
            }
        } catch (error) {
            console.error("Error connecting to MetaMask:", error);
        }
    };

    // New function to trigger the account switching prompt
    const switchMetamaskAccount = async () => {
        if (window.ethereum) {
            try {
                // This re-opens the MetaMask account selection prompt.
                // The `accountsChanged` event listener will handle the state update.
                await window.ethereum.request({
                    method: 'wallet_requestPermissions',
                    params: [{ eth_accounts: {} }],
                });
            } catch (error) {
                console.error('Error requesting permissions:', error);
            }
        }
    };
    // ✅ END: Updated MetaMask Logic

    // Freighter connector
    const connectFreighter = async () => {
        await kit.openModal({
            onWalletSelected: async (opt) => {
                kit.setWallet(opt.id);
                const { address } = await kit.getAddress();
                setFreighterConnected(true);
                setStellarAddress(address);
                const acct = await testnetServer.loadAccount(address);
                setXlmBalance(parseFloat(
                    acct.balances.find(b => b.asset_type === 'native')?.balance || '0'
                ));
            }
        });
    };

    // Reset wallets on pair change
    useEffect(() => {
        setFreighterConnected(false);
        setMetamaskConnected(false);
        setStellarAddress(null);
        setEvmAddress(null);
        setXlmBalance(null);
        setEthBalance(null);
        setFromAmount('');
    }, [tokensPair]);

    // Handler to fill max amount
    const handleUseMax = () => {
        if (tokensPair === 'XLMETH' && xlmBalance != null) {
            setFromAmount(xlmBalance.toString());
        } else if (tokensPair === 'ETHXLM' && ethBalance != null) {
            setFromAmount(ethBalance.toString());
        }
    };

    const submitOrder = async () => {
        const orderData: OrderData = {
            salt: Math.random().toString(36).substring(2, 15),
            src_chain: tokensPair === 'XLMETH' ? 1 : 2,
            dst_chain: tokensPair === 'XLMETH' ? 2 : 1,
            make_amount: fromAmount,
            take_amount: (parseFloat(fromAmount) * 0.95).toString(),
        };
        let signature: Signature = { signed_message: '', signer_address: '' };

        if (tokensPair === 'ETHXLM') {
            if (!window.ethereum) {
                alert('MetaMask not installed');
                return;
            }
            const provider = new ethers.BrowserProvider(window.ethereum);
            const signer = await provider.getSigner();
            const signatureData = await signer.signMessage(JSON.stringify(orderData));
            signature = {
                signed_message: signatureData,
                signer_address: evmAddress || ''
            };
        } else if (tokensPair === 'XLMETH') {
            if (!freighterConnected || !stellarAddress) {
                alert('Freighter not connected');
                return;
            }
            const signatureData = await kit.signMessage(JSON.stringify(orderData));
            signature = {
                signed_message: signatureData.signedMessage,
                signer_address: signatureData.signerAddress || stellarAddress || ''
            };
        }

        const payload: Order = {
            order_data: orderData,
            signature: signature
        };

        try {
            const success = await postOrder(payload); // Send payload directly
            if (!!!success) {
                alert('Failed to submit order');
            }
        } catch (error) {
            console.error('Error submitting order:', error);
            alert('Error submitting order');
        }
    };

    return (
        <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', padding: 2 }}>
            <LocalizationProvider dateAdapter={AdapterDateFns}>
                <Typography variant="h4" gutterBottom>Swap</Typography>
                <Grid container spacing={2} style={{ display: 'flex', flexDirection: 'column', width: '30%' }}>
                    <Grid spacing={12}>
                        <Select
                            labelId="pair-label"
                            fullWidth
                            value={tokensPair}
                            onChange={e => setTokensPair(e.target.value)}
                            renderValue={(selected) => (
                                <Box>
                                    {
                                        selected === 'ETHXLM' ? (
                                            <Box sx={{ display: 'flex', flexDirection: 'row', gap: 2 }}>
                                                <Box component="img" src="/static/images/eth.svg" sx={{ width: 24, height: 24 }} />
                                                <Box>ETH</Box>
                                            </Box>
                                        ) : selected === 'XLMETH' ? (
                                            <Box sx={{ display: 'flex', flexDirection: 'row', gap: 2 }}>
                                                <Box component="img" src="/static/images/xlm.svg" sx={{ width: 24, height: 24 }} />
                                                <Box>XLM</Box>
                                            </Box>
                                        ) : null
                                    }
                                </Box>
                            )}
                        >
                            <MenuItem value="ETHXLM">
                                <ListItemIcon sx={{ minWidth: 36 }}>
                                    <Box
                                        component="img"
                                        src="/static/images/eth.svg"
                                        sx={{ width: 24, height: 24 }}
                                    />
                                </ListItemIcon>
                                <ListItemText primary="ETH" />
                            </MenuItem>
                            <MenuItem value="XLMETH">
                                <ListItemIcon sx={{ minWidth: 36 }}>
                                    <Box
                                        component="img"
                                        src="/static/images/xlm.svg"
                                        sx={{ width: 24, height: 24 }}
                                    />
                                </ListItemIcon>
                                <ListItemText primary="XLM" />
                            </MenuItem>
                        </Select>
                    </Grid>

                    <Grid spacing={6}>
                        <Item>
                            <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'start', gap: 2 }}>
                                {freighterConnected ? (
                                    <Stack direction="row" spacing={1}>
                                        <Button
                                            variant="outlined"
                                            onClick={() => setFreighterConnected(false)}
                                        >
                                            Disconnect
                                        </Button>
                                        <Chip label={stellarAddress} color="primary" />
                                        <Chip label={`${xlmBalance} XLM`} color="success" />
                                    </Stack>
                                ) : (
                                    <Button variant="outlined" onClick={connectFreighter}>
                                        Connect Freighter
                                    </Button>
                                )}
                                {metamaskConnected ? (
                                    <Stack direction="row" spacing={1}>
                                        <Button
                                            variant="outlined"
                                            onClick={switchMetamaskAccount}
                                        >
                                            Switch Account
                                        </Button>
                                        <Chip label={evmAddress} color="primary" />
                                        <Chip label={`${ethBalance?.toFixed(4)} ETH`} color="success" />
                                    </Stack>
                                ) : (
                                    <Button variant="outlined" onClick={connectMetamask}>
                                        Connect MetaMask
                                    </Button>
                                )}
                            </Box>
                            <Stack spacing={2} sx={{ mt: 2 }}>
                                <TextField
                                    label="Amount (XLM)"
                                    type="number"
                                    fullWidth
                                    value={fromAmount}
                                    onChange={e => setFromAmount(e.target.value)}
                                    slotProps={{
                                        input: {
                                            endAdornment: freighterConnected && xlmBalance != null
                                                ? (
                                                    <InputAdornment position="end">
                                                        <Button size="small" onClick={handleUseMax}>
                                                            Max
                                                        </Button>
                                                    </InputAdornment>
                                                )
                                                : undefined
                                        }
                                    }}
                                />

                                <Button
                                    variant="contained"
                                    fullWidth
                                    disabled={!metamaskConnected && !freighterConnected && fromAmount !== ''}
                                    onClick={submitOrder}
                                >
                                    Submit Swap
                                </Button>
                            </Stack>
                        </Item>
                    </Grid>
                    <Grid spacing={6}>
                        <Item>
                            <Stack spacing={2} sx={{ mt: 2 }}>
                                <TextField
                                    label="Amount (ETH)"
                                    type="number"
                                    fullWidth
                                    value={fromAmount}
                                    onChange={e => setFromAmount(e.target.value)}
                                    slotProps={{
                                        input: {
                                            endAdornment: metamaskConnected && ethBalance != null
                                                ? (
                                                    <InputAdornment position="end">
                                                        <Button size="small" onClick={handleUseMax}>
                                                            Max
                                                        </Button>
                                                    </InputAdornment>
                                                )
                                                : undefined
                                        }
                                    }}
                                />
                            </Stack>
                        </Item>
                    </Grid>
                    <Grid spacing={6}>
                        <Item>
                            <TextField
                                label="Resolver Address"
                                type="text"
                                fullWidth
                                value={resolverAddress}
                                onChange={e => setResolverAddress(e.target.value)}
                            />
                        </Item>
                    </Grid>
                    <Accordion
                        sx={{ visibility: freighterConnected || metamaskConnected ? 'visible' : 'hidden' }}
                        expanded={advancedOpen}
                        onChange={(_, isOpen) => setAdvancedOpen(isOpen)}
                    >
                        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                            <Typography>Advanced</Typography>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* advanced form fields */}
                        </AccordionDetails>
                    </Accordion>
                </Grid>

            </LocalizationProvider>
        </Box>
    );
};