from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router_42 = APIRouter()

class Item_42(BaseModel):
    id: int
    name: str

@router_42.get("/items/{item_id}")
async def get_item_42(item_id: int):
    if item_id < 0:
        raise HTTPException(status_code=400, detail="Invalid ID")
    return {"id": item_id, "name": f"Item {item_id}"}