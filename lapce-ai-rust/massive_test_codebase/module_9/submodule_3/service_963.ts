// Service 963
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service963 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}