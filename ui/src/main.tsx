import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { Provider } from 'react-redux';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { store } from './app/store';
import { MainWindow } from './features/main-window/main-window';

import { Buffer } from 'buffer';

declare global {
  interface Window {
    Buffer: typeof Buffer;
  }
}
window.Buffer = Buffer;

export function AppWrapper() {
  const theme = createTheme({
    typography: {
      fontFamily: [
        "Roboto", 
        "Helvetica", 
        "Arial",
        "sans-serif",
      ].join(','),
    },
    palette: {
      primary: {
        main: '#121d4b',
        contrastText: '#ffffff',
      },
      text: {
        primary: '#000000',
        secondary: '#213547',
      },
      background: {
        paper: '#FEFEFE',
        default: '#FFFFFF',
      },
    },
  });

  return (
    <ThemeProvider theme={theme}>
      <MainWindow />
    </ThemeProvider>
  );
}

const container = document.getElementById('root');
const root = createRoot(container!);
root.render(
    <BrowserRouter>
      <Provider store={store}>
        <AppWrapper />
      </Provider>
    </BrowserRouter>
);
