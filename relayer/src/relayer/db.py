
from typing import List
import uuid
from attr import dataclass
import threading

from pydantic import BaseModel

class OrderData(BaseModel):
    salt: str
    src_chain: int
    dst_chain: int
    make_amount: str
    take_amount: str

class Signature(BaseModel):
    signed_message: str
    signer_address: str

class Order(BaseModel):
    order_data: OrderData
    signature: Signature

@dataclass
class InterstellarOrder:
    order_id: str
    hashlock: bytes  # Hash of the secret
    direction: str  # EscrowDirection
    maker: str  # Address
    token: str  # Address
    amount: str  # AmountCalc
    safety_deposit_token: str  # Address
    safety_deposit_amount: int  # i128
    timelocks: dict  # TimeLocks for withdrawal and cancellation

class InterstellarDb:
    _instance = None
    _lock = threading.Lock()

    def __new__(cls, *args, **kwargs):
        if not cls._instance:
            with cls._lock:
                if not cls._instance:
                    cls._instance = super(InterstellarDb, cls).__new__(cls)
        return cls._instance

    def __init__(self):
        if not hasattr(self, "orders"):
            self.orders: List[InterstellarOrder] = []

    def create_order(self, order_data: Order) -> InterstellarOrder:
        """Create a new order and add it to the database."""
        id = str(uuid.uuid4())
        new_order = InterstellarOrder(
            order_id=id,
            hashlock=b'',  # Placeholder for hashlock, should be set appropriately
            direction="EscrowDirection",  # Placeholder, should be set appropriately
            maker=order_data.order_data.salt,  # Placeholder for maker address
            token="",
            amount=order_data.order_data.make_amount,  # Placeholder for amount calculation
            safety_deposit_token="0x",
            safety_deposit_amount=100,
            timelocks={}
        )
        self.orders.append(new_order)
        return new_order

    def read_order(self, order_id: str) -> InterstellarOrder:
        """Retrieve an order by its ID."""
        for order in self.orders:
            if order.order_id == order_id:
                return order
        raise ValueError(f"Order with ID {order_id} not found.")

    def update_order(self, order_id: str, new_order_id: str):
        """Update the order ID of an existing order."""
        for order in self.orders:
            if order.order_id == order_id:
                order.order_id = new_order_id
                return
        raise ValueError(f"Order with ID {order_id} not found.")

    def delete_order(self, order_id: str):
        """Delete an order by its ID."""
        for order in self.orders:
            if order.order_id == order_id:
                self.orders.remove(order)
                return
        raise ValueError(f"Order with ID {order_id} not found.")