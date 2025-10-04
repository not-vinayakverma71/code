// Service 1744
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1744 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}