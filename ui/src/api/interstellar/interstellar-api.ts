import RestApi from '../rest-api';
import { environment } from './../../environments/environment';
import { Order } from './models/order';

export const postOrder = async (order: Order): Promise<boolean> => {
    const client = await getRestClient();
    const response = await client.request('order', 'POST', order); // Send payload directly
    return response.data as boolean;
};

export const postSecret = async (secret: string): Promise<boolean> => {
    const client = await getRestClient();
    const response = await client.request('secret', 'POST', { secret });
    return response.data as boolean;
};

// Rest client
const getRestClient = async (): Promise<RestApi> => {
    const restClient = new RestApi(environment.apiUrl);
    return restClient;
};
