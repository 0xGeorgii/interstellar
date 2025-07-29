import { Grid } from '@mui/material';
import MuiAppBar, { AppBarProps as MuiAppBarProps } from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import CssBaseline from '@mui/material/CssBaseline';
import { styled } from '@mui/material/styles';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import { FC } from 'react';
import { Route, Routes } from 'react-router-dom';
import { useAppSelector } from '../../app/hooks';
import { NoPage } from '../pages/no-page/no-page';
import './main-window.css';
import { selectCurrentPage } from './current-page-slice';
import ErrorDialog from './error-dialog';
import { Swap } from '../pages/swap/swap';


interface AppBarProps extends MuiAppBarProps {
}

const Main = styled('main')<AppBarProps>(({ theme }) => ({
  flexGrow: 1,
  padding: theme.spacing(3),
  transition: theme.transitions.create('margin', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  })
}));

const AppBar = styled(MuiAppBar)<AppBarProps>(({ theme }) => ({
  transition: theme.transitions.create(['margin', 'width'], {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  })
}));

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  padding: theme.spacing(0, 1),
  ...theme.mixins.toolbar,
  justifyContent: 'flex-end',
}));

export const MainWindow: FC = () => {
  const currentPage = useAppSelector(selectCurrentPage);

  return (
    <Box sx={{ height: 'inherit', display: 'flex', position: 'relative' }}>
      <CssBaseline />
      <AppBar position="fixed">
        <Toolbar>
          <Grid container spacing={1}>
            <Grid>
              <Typography variant="h5" noWrap component="div">
                {currentPage.pageName}
              </Typography>
            </Grid>
          </Grid>
            <Typography variant="h5" noWrap component="div" sx={{ textAlign: 'center', flexGrow: 1 }}>
            InterStellar 1Inch DEX
            </Typography>
        </Toolbar>
      </AppBar>
      <Main>
        <DrawerHeader />
        <Routes>
          <Route path="/" element={<Swap />} />
          <Route path="/swap" element={<Swap />} />
          <Route path="*" element={<NoPage />} />
        </Routes>
        <ErrorDialog />
      </Main>
    </Box>
  );
};
