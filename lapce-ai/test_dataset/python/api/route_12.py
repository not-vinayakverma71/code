from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router_12 = APIRouter()

class Item_12(BaseModel):
    id: int
    name: str

@router_12.get("/items/{item_id}")
async def get_item_12(item_id: int):
    if item_id < 0:
        raise HTTPException(status_code=400, detail="Invalid ID")
    return {"id": item_id, "name": f"Item {item_id}"}