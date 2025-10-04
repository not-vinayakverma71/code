import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface State_40 {
    data: any[];
    loading: boolean;
    error: string | null;
}

const slice_40 = createSlice({
    name: 'feature_40',
    initialState: { data: [], loading: false, error: null },
    reducers: {
        setData: (state, action: PayloadAction<any[]>) => {
            state.data = action.payload;
        },
        setLoading: (state, action: PayloadAction<boolean>) => {
            state.loading = action.payload;
        },
    },
});

export const { setData, setLoading } = slice_40.actions;
export default slice_40.reducer;