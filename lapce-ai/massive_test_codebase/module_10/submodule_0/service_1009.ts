// Service 1009
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1009 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}