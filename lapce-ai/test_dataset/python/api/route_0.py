from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router_0 = APIRouter()

class Item_0(BaseModel):
    id: int
    name: str

@router_0.get("/items/{item_id}")
async def get_item_0(item_id: int):
    if item_id < 0:
        raise HTTPException(status_code=400, detail="Invalid ID")
    return {"id": item_id, "name": f"Item {item_id}"}