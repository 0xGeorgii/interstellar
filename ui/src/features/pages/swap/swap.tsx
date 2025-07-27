import { CurrentPageState } from "../../main-window/current-page-slice"

export const Swap: React.FC = () => {
    const currentPageState: CurrentPageState = {
        pageName: 'Swap',
        pageCode: 'swap',
        pageUrl: window.location.pathname,
        routePath: 'swap',
    }

    return (
        <div>
            <h1>Swap Page</h1>
            <p>Current Page: {currentPageState.pageName}</p>
            <p>Page Code: {currentPageState.pageCode}</p>
            <p>Page URL: {currentPageState.pageUrl}</p>
            <p>Route Path: {currentPageState.routePath}</p>
        </div>
    )
}
