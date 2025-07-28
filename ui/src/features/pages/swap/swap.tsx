import { useState } from 'react'

// Extend the Window interface to include the ethereum property
declare global {
    interface Window {
        ethereum?: any;
    }
}
import Button from '@mui/material/Button'
import { Horizon } from "@stellar/stellar-sdk"
import {
    StellarWalletsKit,
    WalletNetwork,
    FreighterModule,
    FREIGHTER_ID,
    ISupportedWallet
} from '@creit.tech/stellar-wallets-kit'

import { CurrentPageState } from "../../main-window/current-page-slice"
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography'
import Grid from '@mui/material/Grid'
import Paper from '@mui/material/Paper'
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import { styled } from '@mui/material/styles'
import { ethers } from 'ethers';
import { showError } from '../../dialog-handler/dialog-handler'
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import InputLabel from '@mui/material/InputLabel';
import TextField from '@mui/material/TextField';

const testnetServer = new Horizon.Server("https://horizon-testnet.stellar.org");

const kit: StellarWalletsKit = new StellarWalletsKit({
    network: WalletNetwork.TESTNET,
    selectedWalletId: FREIGHTER_ID,
    modules: [
        new FreighterModule(),
    ]
});

const Item = styled(Paper)(({ theme }) => ({
    backgroundColor: '#fff',
    ...theme.typography.body2,
    padding: theme.spacing(1),
    textAlign: 'center',
    color: (theme.vars ?? theme).palette.text.secondary,
    ...theme.applyStyles('dark', {
        backgroundColor: '#1A2027',
    }),
}));

export const Swap: React.FC = () => {
    const [freighterConnected, setFreighterConnected] = useState<boolean>(false);
    const [metamaskConnected, setMetamaskConnected] = useState<boolean>(false);
    const [metamaskProvider, setMetamaskProvider] = useState<ethers.BrowserProvider | null>(null);
    const [stellarAddress, setStellarAddress] = useState<string | null>(null);
    const [evmAddress, setEvmAddress] = useState<string | null>(null);
    const [xlmBalance, setXlmBalance] = useState<number | null>(null);
    const [ethBalance, setEthBalance] = useState<number | null>(null);

    const [orderFrom, setOrderFrom] = useState<string>('');

    const [tokensPair, setTokenFrom] = useState<string>('');
    const [fromAmount, setFromAmount] = useState<number>(0);
    const [toAmount, setToAmount] = useState<number>(0);
    const [ethNetwork, setEthNetwork] = useState<string>('Linea');
    const [dueDate, setDueDate] = useState<Date | null>(null);

    const currentPageState: CurrentPageState = {
        pageName: 'Swap',
        pageCode: 'swap',
        pageUrl: window.location.pathname,
        routePath: 'swap',
    }

    async function connectFreighter() {
        try {
            await kit.openModal({
                onWalletSelected: async (option: ISupportedWallet) => {
                    kit.setWallet(option.id);
                    const { address } = await kit.getAddress();
                    setFreighterConnected(true);
                    setStellarAddress(address);
                    const account = await testnetServer.loadAccount(address);
                    setXlmBalance(parseFloat(account.balances.find(balance => balance.asset_type === 'native')?.balance || '0'));
                }
            });
        } catch (error) {
            console.error('Error connecting wallet:', error);
        }
    }

    async function disconnectFreighter() {
        try {
            await kit.disconnect();
            setFreighterConnected(false);
            setStellarAddress(null);
            setXlmBalance(null);
        } catch (error) {
            console.error('Error disconnecting wallet:', error);
        }
    }

    async function connectMetamask() {
        if (window.ethereum == null) {
            showError('MetaMask is not installed. Please install it to connect.');
            return;
        } else {
            let provider = new ethers.BrowserProvider(window.ethereum);
            setMetamaskProvider(provider);
            let signer = await provider.getSigner();
            setEvmAddress(signer.address);
            setMetamaskConnected(true);
            let balance = await provider.getBalance(signer.address);
            setEthBalance(parseFloat(ethers.formatEther(balance)));
        }
    }

    async function disconnectMetamask() {
        setMetamaskProvider(null);
        setMetamaskConnected(false);
        setEvmAddress(null);
        setEthBalance(null);
    }

    return (
        <LocalizationProvider dateAdapter={AdapterDateFns}>
            <Typography variant="h4" gutterBottom>
                Swap
            </Typography>
            <Grid container spacing={2}>
                <Grid size={6}>
                    <Item>
                        <Box>
                            <Box sx={{ textAlign: 'left' }}>
                                {freighterConnected ? (
                                    <Stack direction="row" spacing={1}>
                                        <Button variant="outlined" onClick={async () => await disconnectFreighter()}>
                                            🔗
                                        </Button>
                                        <Chip label={stellarAddress} color="primary" />
                                        <Chip label={xlmBalance} color="success" />
                                    </Stack>
                                ) : (
                                    <Button variant="outlined" onClick={async () => await connectFreighter()}>Connect Freighter</Button>
                                )}
                            </Box>
                            <Box>
                                <Box component="form" sx={{ mt: 2 }}>
                                    <Stack spacing={2}>
                                        <Typography variant="h6">Create Order</Typography>
                                        <Stack direction="row" spacing={2}>
                                            <Box>
                                                <InputLabel id="token-from-label">Pair</InputLabel>
                                                <Select
                                                    labelId="token-from-label"
                                                    id="token-from-select"
                                                    value={tokensPair}
                                                    onChange={(e) => setTokenFrom(e.target.value)}
                                                >
                                                    <MenuItem value="XLMETH">XLM-ETH</MenuItem>
                                                    <MenuItem value="ETHXML">ETH-XML</MenuItem>
                                                </Select>
                                            </Box>
                                            <Box>
                                                <InputLabel id="eth-network-label">ETH Network</InputLabel>
                                                <Select
                                                    labelId="eth-network-label"
                                                    id="eth-network-select"
                                                    value={ethNetwork}
                                                    onChange={(e) => setEthNetwork(e.target.value)}
                                                >
                                                    <MenuItem value="Ethereum">Ethereum</MenuItem>
                                                    <MenuItem value="Linea">Linea</MenuItem>
                                                </Select>
                                            </Box>
                                        </Stack>
                                        <Stack direction="row" spacing={2}>
                                            <Box>
                                                <TextField
                                                    label="From Amount"
                                                    variant="outlined"
                                                    type="number"
                                                    placeholder="Enter amount"
                                                    onChange={(e) => setFromAmount(Number(e.target.value))}
                                                />
                                            </Box>
                                            <Box>
                                                <TextField
                                                    label="To Amount"
                                                    variant="outlined"
                                                    type="number"
                                                    placeholder="Calculated automatically"
                                                    value={toAmount}
                                                    disabled
                                                />
                                            </Box>
                                        </Stack>
                                        <Stack direction="row" spacing={2}>
                                            <Box>
                                                <TextField
                                                    label="Desired Price"
                                                    variant="outlined"
                                                    type="number"
                                                    placeholder="Enter desired price"
                                                />
                                            </Box>
                                            <Box>
                                                <TextField
                                                    label="Result Price"
                                                    variant="outlined"
                                                    type="number"
                                                    placeholder="Calculated automatically"
                                                    disabled
                                                />
                                            </Box>
                                        </Stack>
                                        <Stack direction="row" spacing={2}>
                                            <Box>
                                                <Typography variant="body1">Due</Typography>
                                                <DatePicker
                                                    label="Due Date"
                                                    value={dueDate}
                                                    onChange={(newValue) => setDueDate(newValue)}
                                                    slotProps={{
                                                        textField: {
                                                            size: 'small',
                                                        }
                                                    }}
                                                />
                                            </Box>
                                            <Box>
                                                <TextField
                                                    variant="outlined"
                                                    type="number"
                                                    placeholder="Decay limit in %"
                                                />
                                            </Box>
                                        </Stack>
                                        <Button variant="contained" color="primary">Submit</Button>
                                    </Stack>
                                </Box>
                            </Box>
                        </Box>
                    </Item>
                </Grid>
                <Grid size={6}>
                    <Item>
                        {metamaskConnected ? (
                            <Button variant="outlined" onClick={async () => await disconnectMetamask()}>Disconnect MetaMask</Button>
                        ) : (
                            <Button variant="outlined" onClick={async () => await connectMetamask()}>Connect MetaMask</Button>
                        )}
                        <Typography variant="body1">EVM Address: {evmAddress}</Typography>
                        <Typography variant="body1">ETH Balance: {ethBalance}</Typography>
                    </Item>
                </Grid>
            </Grid>

        </LocalizationProvider>
    )
}
