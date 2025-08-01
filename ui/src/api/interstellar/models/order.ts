export interface OrderData {
    salt: string;
    src_chain: number;
    dst_chain: number;
    make_amount: string;
    take_amount: string;
}

export interface Signature {
    signed_message: string;
    signer_address: string;
}

export interface Order {
    order_data: OrderData;
    signature: Signature;
}
