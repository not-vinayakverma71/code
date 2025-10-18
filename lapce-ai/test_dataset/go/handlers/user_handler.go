package handlers

import (
    "encoding/json"
    "net/http"
    "github.com/gorilla/mux"
)

type UserHandler struct {
    service UserService
}

func (h *UserHandler) GetUser(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    userID := vars["id"]
    
    user, err := h.service.GetUser(userID)
    if err != nil {
        http.Error(w, err.Error(), http.StatusNotFound)
        return
    }
    
    json.NewEncoder(w).Encode(user)
}