from contextlib import asynccontextmanager
import asyncio, logging, os
import dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from rich.logging import RichHandler
import uvicorn


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
            handlers=[RichHandler(rich_tracebacks=True)],
        )
        dotenv.load_dotenv()
        log = logging.getLogger("resolver")
        try:
            pass
        except Exception as _:
            pass
        yield
        print("Shutting down the application")
    except Exception as _:
        pass
    finally:
        pass

app = FastAPI(lifespan=lifespan)
origins = [
    "*",
]

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

class OrderData(BaseModel):
    maker_pk: str
    salt: str
    src_chain: int
    dst_chain: int
    make_amount: str
    take_amount: str

class Order(BaseModel):
    order_data: OrderData
    signature: str

@app.post("/order")
async def available_order(order: Order):
    # Handle order creation
    return {"status": "success", "order": order}

class Secret(BaseModel):
    value: str

@app.post("/secret")
async def create_secret(secret: Secret):
    # Handle secret creation
    return {"status": "success", "secret": secret}

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

if __name__ == "__main__":
    port = int(os.getenv("PORT", "4000"))
    uvicorn.run("src.resolver.main:app", host="0.0.0.0", port=port, reload=True)
